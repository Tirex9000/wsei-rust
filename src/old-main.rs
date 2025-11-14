// use eframe::egui;
// use std::sync::{Arc, Mutex};
// use std::time::Duration;
// use std::thread;

// use sysinfo::*; 

// #[derive(Default, Clone)]
// struct SystemMetrics {
//     cpu_usage: f32,
//     total_memory: u64,
//     used_memory: u64,
//     cpu_name: String,
//     cpu_arch: String,
// }

// struct SystemMonitorApp {
//     metrics: Arc<Mutex<SystemMetrics>>,
// }

// impl Default for SystemMonitorApp {
//     fn default() -> Self {
//         let metrics = Arc::new(Mutex::new(SystemMetrics::default()));
        
//         let metrics_clone = metrics.clone();
//         thread::spawn(move || {
//             let mut sys = System::new();
            
//             // let cpu_name = sys.cpus()
//             //     .get(0)
//             //     .map_or_else(|| "N/A".to_string(), |cpu| cpu.name().to_string());

//             let s = System::new_with_specifics(RefreshKind::nothing().with_cpu(CpuRefreshKind::everything()));

//             let cpu_name = s.cpus()[0].brand().to_string()+" "+&s.cpus()[0].frequency().to_string()+"MHz";
            
//             metrics_clone.lock().unwrap().cpu_name = cpu_name;

//             loop {
//                 sys.refresh_cpu_all(); 
//                 sys.refresh_memory(); 
                
//                 thread::sleep(Duration::from_millis(500));
                
//                 sys.refresh_cpu_all();
                
//                 let cpu_usage = sys.cpus().iter().map(|cpu| cpu.cpu_usage()).sum::<f32>() / sys.cpus().len() as f32;
                
                
//                 let total_memory = sys.total_memory();
//                 let used_memory = sys.used_memory();

//                 let mut current_metrics = metrics_clone.lock().unwrap();
//                 current_metrics.cpu_usage = cpu_usage;
//                 current_metrics.total_memory = total_memory;
//                 current_metrics.used_memory = used_memory;
//                 drop(current_metrics);
                
//                 thread::sleep(Duration::from_millis(500));
//             }
//         });

//         Self { metrics }
//     }
// }

// impl eframe::App for SystemMonitorApp {
//     fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        
//         ctx.request_repaint_after(Duration::from_millis(100));

//         let metrics_lock = self.metrics.lock().unwrap();
//         let current_metrics = metrics_lock.clone();
//         drop(metrics_lock);

//         egui::CentralPanel::default().show(ctx, |ui| {
//             ui.heading("💻 Monitor Systemu Rust");
//             ui.separator();

//             ui.label(format!("⚙️ Procesor: {}", current_metrics.cpu_name));
//             let cpu_percent = current_metrics.cpu_usage.round().min(100.0);
            
//             ui.add(egui::ProgressBar::new(current_metrics.cpu_usage / 100.0)
//                 .text(format!("{:.0}% Zużycia", cpu_percent))
//             );
//             ui.separator();

//             ui.label("💾 Pamięć RAM:");
            
//             const FACTOR: f64 = 1024.0 * 1024.0; 
//             let total_gb = current_metrics.total_memory as f64 / FACTOR;
//             let used_gb = current_metrics.used_memory as f64 / FACTOR;
//             let ram_percent = (current_metrics.used_memory as f32 / current_metrics.total_memory as f32) * 100.0;
            
//             ui.label(format!("Pojemność: {:.2} GB", total_gb));
//             ui.label(format!("Użyte: {:.2} GB", used_gb));
            
//             ui.add(egui::ProgressBar::new(ram_percent / 100.0)
//                 .text(format!("{:.1}% Zużycia RAM", ram_percent))
//             );
//             ui.separator();
//             ui.label("cpu arch: ", current_metrics.cpu_arch);
            
//         });
//     }
// }

// fn main() -> eframe::Result<()> {
//     let options = eframe::NativeOptions {
//         viewport: egui::ViewportBuilder::default().with_inner_size([450.0, 300.0]),
//         ..Default::default()
//     };
    
//     eframe::run_native(
//         "Monitor Systemu Rust",
//         options,
//         Box::new(|_cc| Ok(Box::new(SystemMonitorApp::default()))),
//     )
// }

use eframe::egui;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::thread;
use std::collections::HashMap;

// Używamy zbiorczego importu, aby zapewnić dostępność wszystkich Traitów sysinfo
use sysinfo::*; 


// --- STRUKTURY DANYCH PRZEKAZANE PRZEZ UŻYTKOWNIKA ---

#[derive(Debug, Clone)]
struct CpuCore {
    usage: f32,
    frequency: u64,
}

#[derive(Debug, Clone)]
struct CpuProcessor {
    name: String,
    brand: String,
    arch: String,
    cores: Vec<CpuCore>,

    total_usage: f32,
}

#[derive(Clone, Default)]
struct SystemMetrics {
    os_name: String,
    os_version: String,
    kernel_version: String,
    hostname: String,

    motherboard_name: String,
    motherboard_vendor_name: String,
    motherboard_version: String,
    motherboard_serial_number: String,

    cpus: Vec<CpuProcessor>,

    memory_total: u64, // w MB
    memory_used: u64,  // w MB
    memory_frequency: u64,
}


// --- LOGIKA APLIKACJI GUI ---

/// Główna struktura aplikacji EGUI
struct SystemMonitorApp {
    metrics: Arc<Mutex<SystemMetrics>>,
}

impl Default for SystemMonitorApp {
    fn default() -> Self {
        let metrics = Arc::new(Mutex::new(SystemMetrics::default()));
        
        let metrics_clone = metrics.clone();
        
        // Uruchomienie wątku do ciągłego odświeżania metryk
        thread::spawn(move || {
            let mut sys = System::new_all();

            let m = Motherboard::new();
            let motherboard = m.expect("Nie można uzyskać informacji o płycie głównej.");

            loop {
                // 1. Odświeżenie danych systemowych
                // Używamy refresh_all, by pobrać wszystko, 
                // ale w pętli lepiej używać lżejszych refresh_cpu_all i refresh_memory.
                sys.refresh_cpu_all();
                sys.refresh_memory();
                
                // Krótka pauza (kluczowa, by sysinfo mogło obliczyć usage CPU)
                thread::sleep(Duration::from_millis(500));
                
                sys.refresh_cpu_all();
                
                // 2. Agregacja danych CPU
                let mut cpus_map: HashMap<usize, Vec<&sysinfo::Cpu>> = HashMap::new();

                // sysinfo 0.37.2 zwraca wszystkie rdzenie jako płaską listę. 
                // Pętla musi zgrupować rdzenie według procesorów (jeśli jest ich więcej niż 1 fizyczny).
                // Dla większości systemów wystarczy 0 jako klucz, bo jest tylko 1 fizyczny CPU.
                for (_, cpu) in sys.cpus().iter().enumerate() {
                     // Grupowanie wszystkich rdzeni pod jednym logicznym procesorem (klucz 0)
                     cpus_map.entry(0).or_default().push(cpu); 
                }

                let cpus: Vec<CpuProcessor> = cpus_map.into_iter().map(|(i, cores)| {

                    CpuProcessor {
                        name: cores[i].name().to_string(),
                        brand: cores[i].brand().to_string(),
                        arch: std::env::consts::ARCH.to_string(),
                        cores: cores.iter().map(|c| CpuCore {
                            usage: c.cpu_usage(),
                            frequency: c.frequency(),
                        }).collect(),
                        total_usage: cores[i].cpu_usage(),
                    }
                }).collect();

                // 3. Aktualizacja danych RAM (konwersja na MB)
                let memory_total_mb = sys.total_memory() / 1024 / 1024;
                let memory_used_mb = sys.used_memory() / 1024 / 1024;

                // 4. Bezpieczne zapisanie metryk (blokada Mutex)
                let mut current_metrics = metrics_clone.lock().unwrap();
                current_metrics.os_name = System::name().unwrap_or("N/A".to_string());
                current_metrics.os_version = System::os_version().unwrap_or("N/A".to_string());
                current_metrics.kernel_version = System::kernel_version().unwrap_or("N/A".to_string());
                current_metrics.hostname = System::host_name().unwrap_or("N/A".to_string());

                current_metrics.motherboard_name = motherboard.name().unwrap_or("N/A".to_string());
                current_metrics.motherboard_vendor_name = motherboard.vendor_name().unwrap_or("N/A".to_string());
                current_metrics.motherboard_version = motherboard.version().unwrap_or("N/A".to_string());
                current_metrics.motherboard_serial_number = motherboard.serial_number().unwrap_or("N/A".to_string());

                current_metrics.cpus = cpus;

                current_metrics.memory_total = memory_total_mb;
                current_metrics.memory_used = memory_used_mb;
                current_metrics.memory_frequency = 0; // Brak prostego dostępu do tej danej w sysinfo
                drop(current_metrics);
                
                // Uśpienie wątku
                thread::sleep(Duration::from_millis(500));
            }
        });

        Self { metrics }
    }
}

impl eframe::App for SystemMonitorApp {
    /// Funkcja odpowiadająca za rysowanie interfejsu
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        
        // Wymuszenie ciągłego odświeżania GUI
        ctx.request_repaint_after(Duration::from_millis(100));

        let metrics_lock = self.metrics.lock().unwrap();
        let current_metrics = metrics_lock.clone();
        drop(metrics_lock);
        
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("💻 Monitor Systemu Rust");
            ui.separator();
            


            ui.label("💾 System:");
            ui.label(format!("os_name: {}", current_metrics.os_name));
            ui.label(format!("os_version: {}", current_metrics.os_version));
            ui.label(format!("kernel_version: {}", current_metrics.kernel_version));
            ui.label(format!("hostname: {}", current_metrics.hostname));



            ui.separator();



            ui.label("Płyta główna:");
            ui.label(format!("motherboard_name: {}", current_metrics.motherboard_name));
            ui.label(format!("vendor_name: {}", current_metrics.motherboard_vendor_name));
            ui.label(format!("motherboard_version: {}", current_metrics.motherboard_version));
            ui.label(format!("serial_number: {}", current_metrics.motherboard_serial_number));
    
    

            ui.separator();
            // --- Sekcja CPU ---
            if let Some(cpu) = current_metrics.cpus.first() {
                ui.label(format!("⚙️ Procesor: **{}**", cpu.name));
                ui.label(format!("Model: {}", cpu.brand));
                ui.label(format!("Architektura: {}", cpu.arch));
                ui.label(format!("Rdzenie (logiczne): {}", cpu.cores.len()));

                let cpu_percent = cpu.total_usage.round().min(100.0);
                ui.add_space(5.0);
                
                // Wizualizacja zużycia CPU
                ui.label("Globalne Zużycie CPU:");
                ui.add(egui::ProgressBar::new(cpu.total_usage / 100.0)
                    .text(format!("{:.0}%", cpu_percent))
                );

                ui.separator();

                // --- Sekcja Zużycia Poszczególnych Rdzeni ---
                ui.group(|ui| {
                    ui.set_max_width(200.0);
                    ui.vertical(|ui| {
                        ui.strong("Zużycie Rdzeni:");
                        for (i, core) in cpu.cores.iter().enumerate() {
                            let core_percent = core.usage.round().min(100.0);
                            let freq_mhz = core.frequency / 1000;

                            ui.add(egui::ProgressBar::new(core.usage / 100.0)
                                .text(format!("Rdzeń {}: {:.0}% ({} MHz)", i, core_percent, freq_mhz))
                                .desired_width(ui.available_width())
                            );
                        }
                    });
                });
            } else {
                ui.label("Ładowanie danych CPU...");
            }

            ui.separator();

            // --- Sekcja RAM ---
            ui.label("💾 Pamięć RAM:");
            
            let total_mb = current_metrics.memory_total as f32;
            let used_mb = current_metrics.memory_used as f32;
            let ram_percent = (used_mb / total_mb) * 100.0;
            
            ui.label(format!("Pojemność: {:.1} GB", total_mb / 1024.0));
            ui.label(format!("Użyte: {:.1} GB", used_mb / 1024.0));
            
            // Wizualizacja zużycia RAM
            ui.add(egui::ProgressBar::new(ram_percent / 100.0)
                .text(format!("{:.1}% Zużycia RAM", ram_percent))
            );
            
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([550.0, 600.0]),
        ..Default::default()
    };
    
    eframe::run_native(
        "Monitor Systemu Rust",
        options,
        Box::new(|_cc| Ok(Box::new(SystemMonitorApp::default()))),
    )
}

// fn main() {
//     println!("Hello, world!");
// }
