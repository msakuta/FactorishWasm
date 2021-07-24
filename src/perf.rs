use std::collections::VecDeque;
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

use super::FactorishState;

const MOVING_AVERAGE: usize = 10;
const PERF_HISTORY: usize = 200;

#[derive(Default)]
pub(crate) struct PerfStats {
    values: VecDeque<f64>,
    ma_values: VecDeque<f64>,
    total: f64,
    count: usize,
}

impl PerfStats {
    pub(crate) fn add(&mut self, sample: f64) {
        self.values.push_back(sample);
        while self.values.len() > PERF_HISTORY {
            self.values.pop_front();
        }
        let sum: f64 = self.values.iter().rev().take(MOVING_AVERAGE).sum();
        let len = self.values.iter().rev().take(MOVING_AVERAGE).len();
        self.ma_values.push_back(sum / len as f64);
        while self.ma_values.len() > PERF_HISTORY {
            self.ma_values.pop_front();
        }
        self.total += sample;
        self.count += 1;
    }
}

#[wasm_bindgen]
impl FactorishState {
    pub fn render_perf(&self, context: CanvasRenderingContext2d) -> js_sys::Array {
        let canvas = context.canvas().unwrap();
        let (width, height) = (canvas.width(), canvas.height());
        context.clear_rect(0., 0., width as f64, height as f64);
        context.set_line_width(1.);

        let get_max = |vd: &VecDeque<f64>| vd.iter().fold(1.0f64, |a, b| a.max(*b));
        let get_avg = |vd: &VecDeque<f64>| vd.iter().sum::<f64>() / vd.len() as f64;

        let max = get_max(&self.perf_build_index.values)
            .max(get_max(&self.perf_drop_items.values))
            .max(get_max(&self.perf_simulate.values));

        let plot_series = |vd: &VecDeque<f64>| {
            let mut series = vd.iter();
            context.begin_path();
            series
                .next()
                .map(|p| context.move_to(0., (1. - *p / max) * height as f64));
            for (i, p) in series.enumerate() {
                context.line_to((i + 1) as f64, (1. - *p / max) * height as f64);
            }
            context.stroke();
        };

        context.set_stroke_style(&JsValue::from_str("#00007f"));
        plot_series(&self.perf_build_index.values);

        context.set_stroke_style(&JsValue::from_str("#7f0000"));
        plot_series(&self.perf_drop_items.values);

        context.set_stroke_style(&JsValue::from_str("#007f00"));
        plot_series(&self.perf_simulate.values);

        context.set_stroke_style(&JsValue::from_str("#7f7fff"));
        plot_series(&self.perf_build_index.ma_values);

        context.set_stroke_style(&JsValue::from_str("#ff3f3f"));
        plot_series(&self.perf_drop_items.ma_values);

        context.set_stroke_style(&JsValue::from_str("#00ff00"));
        plot_series(&self.perf_simulate.ma_values);

        js_sys::Array::of4(
            &js_str!("Max: {:.3} ms", max),
            &js_str!(
                "Drop Items Avg: {:.3} ms",
                get_avg(&self.perf_drop_items.values)
            ),
            &js_str!(
                "Build index Avg: {:.3} ms",
                get_avg(&self.perf_build_index.values)
            ),
            &js_str!(
                "Simulate Avg: {:.3} ms",
                get_avg(&self.perf_simulate.values)
            ),
        )
    }
}
