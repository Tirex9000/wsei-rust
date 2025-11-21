// main.rs
use egui::epaint::PathStroke; 
use eframe::egui::{self, Color32}; 
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::thread;
use std::collections::HashMap;

use sysinfo::*; 
use egui::epaint::{Shape, Pos2}; 

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

  // temp: f32,
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
      
      sys.refresh_all(); 

      let motherboard_name = motherboard.name().unwrap_or("N/A".to_string());
      let motherboard_vendor = motherboard.vendor_name().unwrap_or("N/A".to_string());
      let motherboard_version = motherboard.version().unwrap_or("N/A".to_string());
      let motherboard_serial = motherboard.serial_number().unwrap_or("N/A".to_string());
      
      loop {
        thread::sleep(Duration::from_millis(500));
        
        // 1. Odświeżenie danych systemowych
        sys.refresh_cpu_all();
        sys.refresh_memory();
        
        // 2. Agregacja danych CPU
        let mut cpus_map: HashMap<usize, Vec<&sysinfo::Cpu>> = HashMap::new();

        for (_, cpu) in sys.cpus().iter().enumerate() {
            cpus_map.entry(0).or_default().push(cpu); 
        }

        let cpus: Vec<CpuProcessor> = cpus_map.into_iter().map(|(_i, cores)| {
          let first_core = cores.first().expect("Brak rdzeni CPU do przetworzenia!");

          CpuProcessor {
            name: first_core.name().to_string(),
            brand: first_core.brand().to_string(),
            arch: std::env::consts::ARCH.to_string(),
            cores: cores.iter().map(|c| CpuCore {
              usage: c.cpu_usage(),
              frequency: c.frequency(),
            }).collect(),
            total_usage: first_core.cpu_usage(),
          }
        }).collect();

        // --- Dodawanie drugiego procesora do testu ---

        // let mut final_cpus = cpus;

        // if let Some(cpu_data) = final_cpus.first() {
        //   let mut second_cpu = cpu_data.clone();
          
        //   // Zmieniamy nazwę, aby wizualnie odróżnić symulowany procesor
        //   second_cpu.name = format!("{} (Symulowany #2)", second_cpu.name);
          
        //   // Dodajemy sklonowany obiekt do listy
        //   final_cpus.push(second_cpu);
        // }

        // ---------------------------------------------

        // 3. Aktualizacja danych RAM
        let memory_total_mb = sys.total_memory() / 1024 / 1024;
        let memory_used_mb = sys.used_memory() / 1024 / 1024;

        // 4. Bezpieczne zapisanie metryk (blokada Mutex)
        let mut current_metrics = metrics_clone.lock().unwrap();
        current_metrics.os_name = System::name().unwrap_or("N/A".to_string());
        current_metrics.os_version = System::os_version().unwrap_or("N/A".to_string());
        current_metrics.kernel_version = System::kernel_version().unwrap_or("N/A".to_string());
        current_metrics.hostname = System::host_name().unwrap_or("N/A".to_string());

        current_metrics.motherboard_name = motherboard_name.clone();
        current_metrics.motherboard_vendor_name = motherboard_vendor.clone(); 
        current_metrics.motherboard_version = motherboard_version.clone();
        current_metrics.motherboard_serial_number = motherboard_serial.clone();

        current_metrics.cpus = cpus;
        // current_metrics.cpus = final_cpus; // Użycie nowej listy z dwoma procesorami

        current_metrics.memory_total = memory_total_mb;
        current_metrics.memory_used = memory_used_mb;
        current_metrics.memory_frequency = 0; 

        drop(current_metrics);
        
        thread::sleep(Duration::from_millis(500));
      }
    });

    Self { metrics }
  }
}

// Funkcja do rysowania C-kształtnego paska postępu
fn c_shaped_progress_bar(ui: &mut egui::Ui, percentage: f32) -> egui::Response {
    let desired_size = ui.spacing().interact_size.y * 5.0 * egui::vec2(2.0, 1.0); 
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

    if ui.is_rect_visible(rect) {
        let stroke_width = 8.0; 
        
        let radius = rect.height() / 2.0 - stroke_width / 2.0; 
        
        let center = rect.center_bottom() + egui::vec2(0.0, -radius); // center_bottom - radius

        let background_color = ui.visuals().widgets.inactive.bg_fill;
        let fill_color = ui.visuals().widgets.active.fg_stroke.color;

        let num_segments = 30;
        let full_angle = std::f32::consts::PI; // 180 stopni

        let generate_arc_points = |start_angle, end_angle, num_points, radius, center: Pos2| -> Vec<Pos2> {
            let mut points = Vec::with_capacity(num_points);
            for i in 0..=num_points {
                let angle: f32 = start_angle + (end_angle - start_angle) * (i as f32 / num_points as f32);
                let x = center.x + radius * angle.cos();
                let y = center.y + radius * angle.sin();
                points.push(Pos2::new(x, y));
            }
            points
        };
        
        let start_rad = std::f32::consts::PI;
        let end_rad = std::f32::consts::PI * 2.0;
        
        let background_points = generate_arc_points(start_rad, end_rad, num_segments, radius, center);

        ui.painter().add(Shape::Path(egui::epaint::PathShape {
            points: background_points,
            closed: false,
            fill: Color32::TRANSPARENT,
            stroke: PathStroke::new(stroke_width, background_color),
        }));
        
        let fill_end_rad = start_rad + (percentage / 100.0) * full_angle; 
        
        let foreground_points = generate_arc_points(start_rad, fill_end_rad, num_segments, radius, center);

        ui.painter().add(Shape::Path(egui::epaint::PathShape {
            points: foreground_points,
            closed: false,
            fill: Color32::TRANSPARENT,
            stroke: PathStroke::new(stroke_width, fill_color),
        }));

        ui.painter().text(
            center,
            egui::Align2::CENTER_CENTER,
            format!("{:.0}%", percentage),
            egui::FontId::proportional(14.0),
            ui.visuals().text_color(),
        );
    }
    response
}


impl eframe::App for SystemMonitorApp {
  /// Funkcja odpowiadająca za rysowanie interfejsu
  fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
      
      let mut visuals = egui::Visuals::dark();
      visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(30, 30, 30); 
      visuals.widgets.inactive.bg_fill = Color32::from_rgb(40, 40, 40); 
      visuals.widgets.hovered.bg_fill = Color32::from_rgb(60, 60, 60);
      visuals.widgets.active.bg_fill = Color32::from_rgb(80, 80, 80);
      visuals.selection.bg_fill = Color32::from_rgb(0, 120, 215).gamma_multiply(0.5); 
      visuals.override_text_color = Some(Color32::from_rgb(220, 220, 220)); 

      let mut style = (*ctx.style()).clone();
      style.spacing.item_spacing = egui::vec2(8.0, 8.0); 
      style.spacing.window_margin = egui::Margin::same(12);

      style.text_styles.insert(egui::TextStyle::Body, egui::FontId::proportional(15.0));
      style.text_styles.insert(egui::TextStyle::Button, egui::FontId::proportional(15.0));
      style.text_styles.insert(egui::TextStyle::Heading, egui::FontId::proportional(22.0));
      ctx.set_visuals(visuals);
      ctx.set_style(style);


      // Wymuszenie ciągłego odświeżania GUI
      ctx.request_repaint_after(Duration::from_millis(100));

      let metrics_lock = self.metrics.lock().unwrap();
      let current_metrics = metrics_lock.clone();
      drop(metrics_lock);

      egui::CentralPanel::default().show(ctx, |ui| {

        egui::ScrollArea::vertical().show(ui, |ui| {

          ui.vertical_centered_justified(|ui| { 
            ui.add_space(5.0);
            ui.heading("💻 Monitor Systemu Rust");
            ui.add_space(5.0);
          });
          ui.separator();
  
          // --- Sekcja Systemu i Płyty Głównej ---
          ui.columns(2, |columns| {
            // Kolumna 1: System
            columns[0].group(|ui| {
              ui.heading("💾 System");
              ui.add_space(5.0);
              ui.label(format!("**Nazwa OS:** {}", current_metrics.os_name));
              ui.label(format!("**Wersja OS:** {}", current_metrics.os_version));
              ui.label(format!("**Wersja Jądra:** {}", current_metrics.kernel_version));
              ui.label(format!("**Nazwa Hosta:** {}", current_metrics.hostname));
            });

            // Kolumna 2: Płyta Główna
            columns[1].group(|ui| {
              ui.heading(" Płyta Główna");
              ui.add_space(5.0);
              ui.label(format!("**Producent:** {}", current_metrics.motherboard_vendor_name));
              ui.label(format!("**Model:** {}", current_metrics.motherboard_name));
              ui.label(format!("**Wersja:** {}", current_metrics.motherboard_version));
              ui.label(format!("**Numer Seryjny:** {}", current_metrics.motherboard_serial_number));
            });
          });
          ui.separator();
  
  
          // --- Sekcja CPU ---
          if current_metrics.cpus.is_empty() {
            ui.label("Ładowanie danych CPU...");

          } else {
            for cpu in &current_metrics.cpus {
              ui.group(|ui| {
                ui.heading(format!("⚙️ Procesor: **{}**", cpu.name));
                ui.add_space(5.0);
                ui.columns(2, |columns| {
                  columns[0].vertical(|ui| {
                    ui.label(format!("**Marka:** {}", cpu.brand));
                    ui.label(format!("**Architektura:** {}", cpu.arch));
                    ui.label(format!("**Rdzenie (logiczne):** {}", cpu.cores.len()));
                    // ui.label(format!("**Temp:** {}", cpu.temp));
                    ui.add_space(15.0);
                    ui.label("Globalne Zużycie CPU:");
                    c_shaped_progress_bar(ui, cpu.total_usage.round().min(100.0));
                  });
                    
                  // --- Sekcja Zużycia Poszczególnych Rdzeni ---
                  columns[1].vertical_centered_justified(|ui| {
                    ui.heading("📊 Zużycie Rdzeni");
                    ui.add_space(5.0);
                    ui.set_max_width(ui.available_width()); 
                    egui::Grid::new(cpu.name.clone()+"core_usage_grid").num_columns(1).spacing([10.0, 4.0]).show(ui, |ui| {
                      for (i, core) in cpu.cores.iter().enumerate() {
                        let core_percent = core.usage.round().min(100.0);
                        let freq_mhz = core.frequency / 1000;

                        ui.add(egui::ProgressBar::new(core.usage / 100.0)
                          .text(format!("Rdzeń {}: {:.0}% ({} MHz)", i, core_percent, freq_mhz))
                          .desired_width(ui.available_width())
                          // .fill(ui.visuals().widgets.active.fg_stroke.color) 
                        );
                        ui.end_row();
                      }
                    });
                  });
                });
              });
              ui.separator();
            }
          }
          ui.separator();
  
          // --- Sekcja RAM ---
          ui.group(|ui| {
            ui.heading("💾 Pamięć RAM");
            ui.add_space(5.0);

            let total_mb = current_metrics.memory_total as f32;
            let used_mb = current_metrics.memory_used as f32;
            let free_mb = current_metrics.memory_total as f32 - used_mb;
            let ram_percent = (used_mb / total_mb) * 100.0;
            let free_percent = 100.0 - ram_percent;

            ui.columns(2, |columns| {
              columns[0].vertical(|ui| {
                ui.label(format!("**Pojemność:** {:.1} GB", total_mb / 1024.0));
                ui.label(format!("**Użyte:** {:.1} GB ({:.1}%)", used_mb / 1024.0, ram_percent));
                ui.label(format!("**Wolne:** {:.1} GB ({:.1}%)", free_mb / 1024.0, free_percent));
              });
              
              columns[1].vertical_centered_justified(|ui| {
                ui.label("Zużycie RAM:");
                c_shaped_progress_bar(ui, ram_percent.round().min(100.0));
              });
            });
          });

        }); // <<-- Koniec bloku ScrollArea

      });
  }
}

fn main() -> eframe::Result<()> {
  let options = eframe::NativeOptions {
    viewport: egui::ViewportBuilder::default()
      .with_inner_size([1280.0, 720.0]) 
      // .with_min_inner_size([650.0, 750.0]) // Ustawienie stałego rozmiaru min = max
      // .with_max_inner_size([650.0, 750.0])
      .with_resizable(true),
    ..Default::default()
  };

  eframe::run_native(
    "Monitor Systemu Rust",
    options,
    Box::new(|_cc| Ok(Box::new(SystemMonitorApp::default()))),
  )
}