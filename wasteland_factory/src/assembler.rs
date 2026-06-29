use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assembler {
    pub id: String,
    pub recipe: Option<AssemblyRecipe>,
    pub input_buffer: Vec<AssemblyInput>,
    pub output_buffer: Vec<AssemblyOutput>,
    pub progress: f32,
    pub speed: f32,
    pub precision: f32,
    pub power_consumption: f32,
    pub quality_tolerance: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssemblyRecipe {
    pub name: String,
    pub inputs: Vec<RecipeInput>,
    pub outputs: Vec<RecipeOutput>,
    pub base_time: f32,
    pub min_precision: f32,
    pub temperature_required: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeInput {
    pub material_id: String,
    pub quantity: f32,
    pub acceptable_tolerance: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeOutput {
    pub product_id: String,
    pub quantity: f32,
    pub quality_dependencies: Vec<QualityDependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityDependency {
    pub property: String,
    pub weight: f32,
    pub input_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssemblyInput {
    pub material_id: String,
    pub quantity: f32,
    pub hardness: f32,
    pub purity: f32,
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssemblyOutput {
    pub product_id: String,
    pub quantity: f32,
    pub quality: f32,
    pub defects: Vec<String>,
}

impl Assembler {
    pub fn new(speed: f32, precision: f32) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            recipe: None,
            input_buffer: Vec::new(),
            output_buffer: Vec::new(),
            progress: 0.0,
            speed,
            precision,
            power_consumption: 100.0,
            quality_tolerance: 0.95,
        }
    }

    pub fn set_recipe(&mut self, recipe: AssemblyRecipe) {
        self.recipe = Some(recipe);
        self.progress = 0.0;
    }

    pub fn add_input(&mut self, input: AssemblyInput) {
        self.input_buffer.push(input);
    }

    pub fn can_produce(&self) -> bool {
        if let Some(recipe) = &self.recipe {
            for required in &recipe.inputs {
                let available: f32 = self
                    .input_buffer
                    .iter()
                    .filter(|i| i.material_id == required.material_id)
                    .map(|i| i.quantity)
                    .sum();
                if available < required.quantity {
                    return false;
                }
            }
            true
        } else {
            false
        }
    }

    pub fn update(&mut self, dt: f32) {
        if !self.can_produce() {
            return;
        }

        let recipe = self.recipe.as_ref().unwrap().clone();
        let effective_speed = self.speed * (1.0 - (1.0 - self.precision) * 0.5);
        self.progress += effective_speed * dt / recipe.base_time;

        if self.progress >= 1.0 {
            self.progress = 0.0;
            self.produce(&recipe);
        }
    }

    fn produce(&mut self, recipe: &AssemblyRecipe) {
        let mut output_quality = 1.0;
        let mut defects = Vec::new();

        for required in &recipe.inputs {
            let mut consumed = 0.0;
            self.input_buffer.retain(|input| {
                if input.material_id == required.material_id && consumed < required.quantity {
                    consumed += input.quantity;
                    if input.purity < self.quality_tolerance {
                        output_quality *= input.purity;
                        defects.push(format!("low_purity_{}", input.material_id));
                    }
                    if input.hardness < required.acceptable_tolerance {
                        output_quality *= 0.95;
                    }
                    false
                } else {
                    true
                }
            });
        }

        let precision_penalty = (1.0 - self.precision) * 0.2;
        output_quality = (output_quality - precision_penalty).max(0.0);

        for output in &recipe.outputs {
            self.output_buffer.push(AssemblyOutput {
                product_id: output.product_id.clone(),
                quantity: output.quantity,
                quality: output_quality,
                defects: defects.clone(),
            });
        }
    }

    pub fn take_outputs(&mut self) -> Vec<AssemblyOutput> {
        let outputs = self.output_buffer.clone();
        self.output_buffer.clear();
        outputs
    }

    pub fn efficiency(&self) -> f32 {
        let recipe = match &self.recipe {
            Some(r) => r,
            None => return 0.0,
        };
        let total_inputs: f32 = recipe.inputs.iter().map(|i| i.quantity).sum();
        let total_outputs: f32 = recipe.outputs.iter().map(|o| o.quantity).sum();
        if total_inputs > 0.0 { total_outputs / total_inputs } else { 0.0 }
    }
}
