use anyhow::{Context, Result};
use std::collections::HashSet;
use std::io::{self, Write};
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug)]
pub(crate) struct StepController {
    enabled: bool,
    counter: AtomicUsize,
    skip_steps: HashSet<usize>,
}

impl StepController {
    pub(crate) fn new(enabled: bool, skip_steps: &[usize]) -> Self {
        Self {
            enabled,
            counter: AtomicUsize::new(0),
            skip_steps: skip_steps.iter().copied().collect(),
        }
    }

    pub(crate) fn enabled(&self) -> bool {
        self.enabled
    }

    pub(crate) fn pause(&self, description: &str) -> Result<()> {
        let step_number = self.counter.fetch_add(1, Ordering::SeqCst) + 1;
        if !self.enabled {
            return Ok(());
        }

        if self.skip_steps.contains(&step_number) {
            crate::log_info!("Skipping interactive step {}: {}", step_number, description);
            println!("\n--- Skipping Step {}: {}", step_number, description);
            return Ok(());
        }

    crate::log_info!("Interactive step {}: {}", step_number, description);
        println!("\n=== Step {}: {} ===", step_number, description);
        print!("Press Enter to continue...");
        io::stdout()
            .flush()
            .context("Failed to flush stdout during step-through pause")?;
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .context("Failed to read input during step-through pause")?;
        Ok(())
    }
}
