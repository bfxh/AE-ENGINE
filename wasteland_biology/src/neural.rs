use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Neuron {
    pub id: Uuid,
    pub threshold: f32,
    pub firing_rate: f32,
    pub neurotransmitter: String,
    pub connections: Vec<(Uuid, f32)>,
    pub potential: f32,
    pub last_fired: f32,
    pub refractory_period: f32,
    pub adaptation: f32,
    pub noise: f32,
}

impl Neuron {
    pub fn new(threshold: f32, firing_rate: f32) -> Self {
        Self {
            id: Uuid::new_v4(),
            threshold,
            firing_rate,
            neurotransmitter: "glutamate".to_string(),
            connections: Vec::new(),
            potential: 0.0,
            last_fired: -1.0,
            refractory_period: 0.002,
            adaptation: 0.0,
            noise: 0.01,
        }
    }

    pub fn with_neurotransmitter(mut self, nt: &str) -> Self {
        self.neurotransmitter = nt.to_string();
        self
    }

    pub fn fire(&mut self, current_time: f32) -> bool {
        if current_time - self.last_fired < self.refractory_period {
            return false;
        }

        let effective_threshold = self.threshold + self.adaptation;
        if self.potential >= effective_threshold {
            self.potential = 0.0;
            self.last_fired = current_time;
            self.adaptation += 0.05;
            true
        } else {
            false
        }
    }

    pub fn receive_signal(&mut self, strength: f32) {
        self.potential += strength;
        self.potential = self.potential.max(0.0);
    }

    pub fn decay(&mut self, dt: f32, decay_rate: f32) {
        self.potential *= (1.0 - decay_rate * dt).max(0.0);
        self.adaptation *= (1.0 - 0.1 * dt).max(0.0);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuralNetwork {
    pub neurons: Vec<Neuron>,
    pub input_size: usize,
    pub output_size: usize,
    pub hidden_layers: Vec<usize>,
    pub learning_rate: f32,
    pub decay_rate: f32,
    pub time: f32,
}

impl NeuralNetwork {
    pub fn new(input_size: usize, output_size: usize, hidden_layers: Vec<usize>) -> Self {
        Self {
            neurons: Vec::new(),
            input_size,
            output_size,
            hidden_layers,
            learning_rate: 0.01,
            decay_rate: 0.1,
            time: 0.0,
        }
    }

    pub fn create_simple_brain(&mut self, body_part_count: usize) -> Vec<Uuid> {
        let mut ids = Vec::new();

        let motor_count = body_part_count;
        let sensory_count = body_part_count;

        for _ in 0..sensory_count {
            let neuron = Neuron::new(0.3, 10.0);
            ids.push(neuron.id);
            self.neurons.push(neuron);
        }

        let hidden_count = self.hidden_layers.iter().sum::<usize>().max(4);
        let mut hidden_ids = Vec::new();
        for _ in 0..hidden_count {
            let neuron = Neuron::new(0.5, 5.0);
            let id = neuron.id;
            hidden_ids.push(id);
            self.neurons.push(neuron);
        }

        let mut motor_ids = Vec::new();
        for _ in 0..motor_count {
            let neuron = Neuron::new(0.6, 8.0);
            let id = neuron.id;
            motor_ids.push(id);
            self.neurons.push(neuron);
        }

        let mut rng = rand::thread_rng();
        use rand::Rng;

        for i in 0..sensory_count {
            for &h_id in &hidden_ids {
                let weight = rng.gen_range(-0.5..0.5);
                self.neurons[i].connections.push((h_id, weight));
            }
        }

        for &h_id in &hidden_ids {
            for &m_id in &motor_ids {
                let weight = rng.gen_range(-0.5..0.5);
                if let Some(neuron) = self.neurons.iter_mut().find(|n| n.id == h_id) {
                    neuron.connections.push((m_id, weight));
                }
            }
        }

        for i in 0..hidden_ids.len() {
            for j in (i + 1)..hidden_ids.len() {
                let weight = rng.gen_range(-0.3..0.3);
                if let Some(neuron) = self.neurons.iter_mut().find(|n| n.id == hidden_ids[i]) {
                    neuron.connections.push((hidden_ids[j], weight));
                }
            }
        }

        ids
    }

    pub fn propagate(&mut self, inputs: &[f32]) -> Vec<f32> {
        self.time += 0.001;

        let input_end = self.input_size.min(inputs.len());
        let neuron_count = self.neurons.len();

        for (i, input) in inputs.iter().enumerate().take(input_end) {
            if i < neuron_count {
                self.neurons[i].receive_signal(*input);
            }
        }

        let mut fired = Vec::new();
        for i in 0..neuron_count {
            if self.neurons[i].fire(self.time) {
                fired.push(i);
            }
        }

        let mut signals: Vec<(usize, usize, f32)> = Vec::new();
        for &idx in &fired {
            let connections = self.neurons[idx].connections.clone();
            for (target_id, weight) in &connections {
                if let Some(target_idx) = self.neurons.iter().position(|n| n.id == *target_id) {
                    let signal = self.neurons[idx].firing_rate * weight;
                    signals.push((idx, target_idx, signal));
                }
            }
        }

        for (_, target_idx, signal) in &signals {
            self.neurons[*target_idx].receive_signal(*signal);
        }

        for neuron in &mut self.neurons {
            neuron.decay(0.001, self.decay_rate);
        }

        let output_start = neuron_count.saturating_sub(self.output_size);
        let outputs: Vec<f32> = self.neurons[output_start..]
            .iter()
            .map(|n| if n.potential > n.threshold { n.potential } else { 0.0 })
            .collect();

        outputs
    }

    pub fn learn(&mut self, reward: f32) {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        let adjustment = self.learning_rate * reward;

        for neuron in &mut self.neurons {
            for (_, weight) in &mut neuron.connections {
                let noise = rng.gen_range(-0.001..0.001);
                *weight += adjustment * (*weight).signum() * 0.01 + noise;
                *weight = weight.clamp(-1.0, 1.0);
            }
        }
    }

    pub fn get_activation(&self, _pattern: &[f32]) -> Vec<f32> {
        let mut activations = Vec::new();
        for neuron in &self.neurons {
            let mut total_input = neuron.potential;
            for (conn_id, weight) in &neuron.connections {
                if let Some(idx) = self.neurons.iter().position(|n| n.id == *conn_id) {
                    total_input += self.neurons[idx].potential * weight;
                }
            }
            let activation = if total_input > neuron.threshold {
                total_input / (1.0 + total_input.abs())
            } else {
                0.0
            };
            activations.push(activation);
        }
        activations
    }

    pub fn reset(&mut self) {
        self.time = 0.0;
        for neuron in &mut self.neurons {
            neuron.potential = 0.0;
            neuron.adaptation = 0.0;
            neuron.last_fired = 0.0;
        }
    }

    pub fn neuron_count(&self) -> usize {
        self.neurons.len()
    }

    pub fn connection_count(&self) -> usize {
        self.neurons.iter().map(|n| n.connections.len()).sum()
    }
}

impl Default for NeuralNetwork {
    fn default() -> Self {
        Self::new(4, 4, vec![8, 8])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neuron_fire() {
        let mut neuron = Neuron::new(0.5, 10.0);
        neuron.receive_signal(1.0);
        assert!(neuron.fire(0.0));
        assert_eq!(neuron.potential, 0.0);
    }

    #[test]
    fn test_neuron_refractory() {
        let mut neuron = Neuron::new(0.5, 10.0);
        neuron.receive_signal(1.0);
        assert!(neuron.fire(0.0));
        neuron.receive_signal(1.0);
        assert!(!neuron.fire(0.001));
    }

    #[test]
    fn test_create_simple_brain() {
        let mut nn = NeuralNetwork::new(6, 6, vec![8, 4]);
        let ids = nn.create_simple_brain(6);
        assert_eq!(ids.len(), 6);
        assert!(nn.neuron_count() > 12);
        assert!(nn.connection_count() > 0);
    }

    #[test]
    fn test_propagate() {
        let mut nn = NeuralNetwork::new(4, 2, vec![4]);
        nn.create_simple_brain(4);

        let inputs = vec![0.5, 0.3, 0.8, 0.1];
        let outputs = nn.propagate(&inputs);
        assert!(!outputs.is_empty());
    }

    #[test]
    fn test_learn() {
        let mut nn = NeuralNetwork::new(4, 2, vec![4]);
        nn.create_simple_brain(4);

        let before = nn.propagate(&[0.5, 0.3, 0.8, 0.1]);
        nn.learn(1.0);
        let after = nn.propagate(&[0.5, 0.3, 0.8, 0.1]);

        assert_eq!(before.len(), after.len());
    }

    #[test]
    fn test_reset() {
        let mut nn = NeuralNetwork::new(4, 2, vec![4]);
        nn.create_simple_brain(4);
        nn.propagate(&[0.5, 0.3, 0.8, 0.1]);
        nn.reset();

        for neuron in &nn.neurons {
            assert_eq!(neuron.potential, 0.0);
        }
    }
}
