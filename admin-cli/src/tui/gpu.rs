use serde_json::Value;
use std::collections::BTreeMap;

/// Bundled GPU data from gpus.json
const GPU_DATA: &str = include_str!("../../../gpus.json");

/// A GPU model available on the network
#[derive(Debug, Clone)]
pub struct GpuModel {
    pub name: String,
    pub interface: String,
    pub memory_size: String,
    pub vendor: String,
}

/// GPU catalog loaded from gpus.json
pub struct GpuCatalog {
    pub models: Vec<GpuModel>,
    /// Deduplicated list of model names (for SDL selection)
    pub unique_models: Vec<GpuModelSummary>,
}

/// Summary of a GPU model (deduplicated across variants)
#[derive(Debug, Clone)]
pub struct GpuModelSummary {
    pub name: String,
    pub vendor: String,
    pub variants: Vec<GpuVariant>,
    pub selected: bool,
}

#[derive(Debug, Clone)]
pub struct GpuVariant {
    pub interface: String,
    pub memory_size: String,
}

impl GpuCatalog {
    pub fn load() -> Self {
        let models = Self::parse_gpu_data();
        let unique_models = Self::deduplicate(&models);
        Self { models, unique_models }
    }

    fn parse_gpu_data() -> Vec<GpuModel> {
        let doc: Value = serde_json::from_str(GPU_DATA).unwrap_or(Value::Null);
        let mut models = Vec::new();

        if let Value::Object(vendors) = &doc {
            for (_vendor_id, vendor_data) in vendors {
                let vendor_name = vendor_data.get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                if let Some(Value::Object(devices)) = vendor_data.get("devices") {
                    for (_device_id, device_data) in devices {
                        let name = device_data.get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string();
                        let interface = device_data.get("interface")
                            .and_then(|v| v.as_str())
                            .unwrap_or("PCIe")
                            .to_string();
                        let memory_size = device_data.get("memory_size")
                            .and_then(|v| v.as_str())
                            .unwrap_or("0Gi")
                            .to_string();

                        models.push(GpuModel {
                            name,
                            interface,
                            memory_size,
                            vendor: vendor_name.clone(),
                        });
                    }
                }
            }
        }

        // Sort by name then memory
        models.sort_by(|a, b| a.name.cmp(&b.name).then(a.memory_size.cmp(&b.memory_size)));
        models
    }

    fn deduplicate(models: &[GpuModel]) -> Vec<GpuModelSummary> {
        let mut grouped: BTreeMap<String, GpuModelSummary> = BTreeMap::new();

        for model in models {
            let entry = grouped.entry(model.name.clone()).or_insert_with(|| {
                GpuModelSummary {
                    name: model.name.clone(),
                    vendor: model.vendor.clone(),
                    variants: Vec::new(),
                    selected: false,
                }
            });

            let variant = GpuVariant {
                interface: model.interface.clone(),
                memory_size: model.memory_size.clone(),
            };

            // Avoid duplicate variants
            if !entry.variants.iter().any(|v| v.interface == variant.interface && v.memory_size == variant.memory_size) {
                entry.variants.push(variant);
            }
        }

        grouped.into_values().collect()
    }

    /// Get the list of selected GPU model names (for SDL generation)
    pub fn selected_models(&self) -> Vec<&str> {
        self.unique_models.iter()
            .filter(|m| m.selected)
            .map(|m| m.name.as_str())
            .collect()
    }

    /// Toggle selection of a GPU model by index
    pub fn toggle(&mut self, index: usize) {
        if let Some(model) = self.unique_models.get_mut(index) {
            model.selected = !model.selected;
        }
    }

    /// Pre-select models that match the current SDL GPU list
    pub fn select_from_sdl(&mut self, model_names: &[&str]) {
        for model in &mut self.unique_models {
            model.selected = model_names.contains(&model.name.as_str());
        }
    }

    /// Filter models by minimum memory (parses "XGi" suffix)
    pub fn models_with_min_memory(&self, min_gi: u64) -> Vec<&GpuModelSummary> {
        self.unique_models.iter()
            .filter(|m| {
                m.variants.iter().any(|v| {
                    parse_gi(&v.memory_size) >= min_gi
                })
            })
            .collect()
    }
}

fn parse_gi(s: &str) -> u64 {
    s.trim_end_matches("Gi")
        .trim_end_matches("Mi")
        .parse::<u64>()
        .unwrap_or(0)
}
