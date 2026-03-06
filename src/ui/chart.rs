use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use egui_plot::{Plot, Bar, BarChart};

use crate::simulation::{InfectionTimeSeries, SimulationTime};

pub fn infection_chart_ui(
    mut contexts: EguiContexts,
    time_series: Res<InfectionTimeSeries>,
    sim_time: Res<SimulationTime>,
) {
    let ctx = contexts.ctx_mut();

    let screen = ctx.screen_rect();
    egui::Window::new("Daily New Infections")
        .default_pos(egui::pos2(screen.max.x - 620.0, screen.max.y - 220.0))
        .default_size(egui::vec2(600.0, 200.0))
        .collapsible(true)
        .resizable(true)
        .show(ctx, |ui| {
            let n = time_series.daily_wpv.len();
            if n == 0 {
                ui.label(format!("Day {} — no transmission data yet", sim_time.day));
                return;
            }

            let mut wpv_bars = Vec::with_capacity(n);
            let mut vdpv_bars = Vec::with_capacity(n);
            let mut opv_bars = Vec::with_capacity(n);

            for i in 0..n {
                let day = (time_series.start_day + i as u32) as f64;
                let wpv = time_series.daily_wpv[i] as f64;
                let vdpv = time_series.daily_vdpv[i] as f64;
                let opv = time_series.daily_opv[i] as f64;

                wpv_bars.push(Bar::new(day, wpv).width(0.8));
                vdpv_bars.push(Bar::new(day, vdpv).width(0.8).base_offset(wpv));
                opv_bars.push(Bar::new(day, opv).width(0.8).base_offset(wpv + vdpv));
            }

            let wpv_chart = BarChart::new(wpv_bars)
                .color(egui::Color32::from_rgb(230, 38, 38))
                .name("WPV");
            let vdpv_chart = BarChart::new(vdpv_bars)
                .color(egui::Color32::from_rgb(255, 153, 0))
                .name("VDPV");
            let opv_chart = BarChart::new(opv_bars)
                .color(egui::Color32::from_rgb(0, 217, 217))
                .name("OPV");

            Plot::new("infection_timeseries")
                .legend(egui_plot::Legend::default())
                .x_axis_label("Day")
                .y_axis_label("New infections")
                .auto_bounds([true, true].into())
                .height(ui.available_height().max(120.0))
                .show(ui, |plot_ui| {
                    plot_ui.bar_chart(wpv_chart);
                    plot_ui.bar_chart(vdpv_chart);
                    plot_ui.bar_chart(opv_chart);
                });
        });
}
