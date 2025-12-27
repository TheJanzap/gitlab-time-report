//! Contains traits and helper functions for creating charts with [`charming`].

// Exclude file from testing, as we would be testing the library implementation.
#![cfg(not(tarpaulin_include))]

use super::{ChartSettingError, round_to_string};
use crate::TimeDeltaExt;
use crate::filters::total_time_spent;
use crate::model::TimeLog;
use charming::component::{
    DataView, Feature, Grid, MagicType, MagicTypeType, Restore, SaveAsImage, Toolbox,
};
use charming::datatype::DataPoint;
use charming::element::{AxisLabel, AxisType, JsFunction, Orient, Tooltip};
use charming::theme::Theme;
use charming::{Chart, component, element, series};
use std::collections::BTreeMap;

/// Trait for all charming series
pub(super) trait Series {
    /// Turns a grouped time log map into entries of total hours per key and the key itself.
    /// Can then be converted to [`DataPoint`] for chart data.
    fn create_data_point_mapping<'a, T>(
        grouped_time_log: BTreeMap<impl Into<Option<&'a T>>, Vec<&'a TimeLog>>,
    ) -> Vec<(String, String)>
    where
        T: std::fmt::Display + 'a,
    {
        grouped_time_log
            .into_iter()
            .map(|(key, time_logs)| {
                let total_hours = round_to_string(
                    total_time_spent(time_logs).total_hours(),
                    super::ROUNDING_PRECISION,
                );
                let key = Self::option_to_string(key);
                (total_hours, key)
            })
            .collect()
    }

    /// Converts an optional key to a string. If the key is `None`, the string "None" is returned.
    fn option_to_string<'a, T>(key: impl Into<Option<T>>) -> String
    where
        T: std::fmt::Display + 'a,
    {
        match key.into() {
            Some(k) => k.to_string(),
            None => "None".to_string(),
        }
    }
}
/// Trait for chart types that require one series for each data point.
pub(super) trait MultiSeries: Series {
    fn with_defaults<D: Into<DataPoint>>(key: &str, data: Vec<D>) -> Self;
}

/// Trait for chart types that require one series for all data points.
pub(super) trait SingleSeries: Series {
    fn with_defaults<D: Into<DataPoint>>(data: Vec<D>) -> Self;
}

impl Series for series::Bar {}
impl MultiSeries for series::Bar {
    /// Creates a new [`series::Bar`] with the given key and data.
    fn with_defaults<D: Into<DataPoint>>(key: &str, data: Vec<D>) -> Self {
        series::Bar::new()
            .name(key)
            .data(data)
            .label(element::Label::new()
                .show(true)
                .position(element::LabelPosition::Top)
            )
    }
}

impl Series for series::Line {}
impl MultiSeries for series::Line {
    fn with_defaults<D: Into<DataPoint>>(key: &str, data: Vec<D>) -> Self {
        series::Line::new().name(key).data(data).symbol_size(8)
    }
}

impl Series for series::Pie {}
impl SingleSeries for series::Pie {
    /// Creates a new [`series::Pie`] with the given data.
    fn with_defaults<D: Into<DataPoint>>(data: Vec<D>) -> Self {
        series::Pie::new()
            .data(data)
            .label(element::Label::new().formatter("{b}\n{c}h"))
            .radius("70%")
    }
}

/// Extension methods for [`charming::Chart`]
pub(super) trait ChartExt {
    fn with_defaults(title: &str) -> Self;
    fn with_axes(self, x_axis_label: &[String], x_axis_label_rotate: f64) -> Self;
    fn with_legend(self) -> Self;
    fn with_toolbox(self, features: Feature) -> Self;

    fn create_bar_chart(
        series: Vec<series::Bar>,
        x_axis_label: &[String],
        x_axis_label_rotate: f64,
        title: &str,
    ) -> Self
    where
        Self: Sized;

    fn create_line_chart(
        grouped_time_log: Vec<series::Line>,
        x_axis_label: &[String],
        x_axis_label_rotate: f64,
        title: &str,
    ) -> Self
    where
        Self: Sized;

    fn create_pie_chart(series: series::Pie, title: &str) -> Self
    where
        Self: Sized;

    fn render_svg(
        &self,
        width: u32,
        height: u32,
        custom_theme: Option<&str>,
    ) -> Result<String, ChartSettingError>;
    fn render_html(
        &self,
        width: u64,
        height: u64,
        custom_theme: Option<&str>,
    ) -> Result<String, ChartSettingError>;
}

impl ChartExt for Chart {
    /// Creates a new chart with default settings
    fn with_defaults(title: &str) -> Chart {
        Chart::new()
            .title(component::Title::new().text(title).left("center"))
            .tooltip(
                Tooltip::new()
                    .trigger(element::Trigger::Item)
                    .value_formatter(JsFunction::new_with_args(
                        "value",
                        "return String(value).concat('h');",
                    )),
            )
    }

    /// Adds x and y-axis to the chart.
    fn with_axes(self, x_axis_label: &[String], x_axis_label_rotate: f64) -> Self {
        self.y_axis(
            component::Axis::new()
                .type_(AxisType::Value)
                .name("Hours".to_string())
                .split_number(10),
        )
        .x_axis(
            component::Axis::new()
                .type_(AxisType::Category)
                .data(x_axis_label.to_vec())
                .axis_label(AxisLabel::new().rotate(x_axis_label_rotate)),
        )
    }

    /// Adds a legend to the chart.
    fn with_legend(self) -> Self {
        self.legend(component::Legend::new().top("7%").width("80%"))
    }

    /// Adds a toolbox with the specified features to the chart.
    fn with_toolbox(self, feature: Feature) -> Self {
        self.toolbox(
            Toolbox::new()
                .orient(Orient::Vertical)
                .top("center")
                .feature(feature),
        )
    }

    /// Creates a new bar chart with the given series, x-axis label, x-axis rotation value and title.
    fn create_bar_chart(
        series: Vec<series::Bar>,
        x_axis_label: &[String],
        x_axis_label_rotate: f64,
        title: &str,
    ) -> Self {
        let mut chart = Chart::with_defaults(title)
            .grid(
                Grid::new()
                    // More space for the legend
                    .bottom("5%")
                    .top("20%")
                    .left("2%")
                    .right("5%")
                    .contain_label(true),
            )
            .with_axes(x_axis_label, x_axis_label_rotate)
            .with_legend()
            .with_toolbox(
                Feature::new()
                    .save_as_image(SaveAsImage::new())
                    .magic_type(
                        MagicType::new().type_(vec![MagicTypeType::Stack, MagicTypeType::Line]),
                    )
                    .restore(Restore::new())
                    .data_view(DataView::new()),
            );

        for bar in series {
            chart = chart.series(bar);
        }
        chart
    }

    /// Creates a new line chart with the given series, x-axis label, x-axis rotation value and title.
    fn create_line_chart(
        lines: Vec<series::Line>,
        x_axis_label: &[String],
        x_axis_label_rotate: f64,
        title: &str,
    ) -> Self {
        let mut chart = Chart::with_defaults(title)
            .grid(
                Grid::new()
                    // More space for the legend
                    .bottom("5%")
                    .top("20%")
                    .left("2%")
                    .right("5%")
                    .contain_label(true),
            )
            .with_axes(x_axis_label, x_axis_label_rotate)
            .with_legend()
            .with_toolbox(
                Feature::new()
                    .save_as_image(SaveAsImage::new())
                    .magic_type(MagicType::new().type_(vec![MagicTypeType::Bar]))
                    .restore(Restore::new())
                    .data_view(DataView::new()),
            );

        for bar in lines {
            chart = chart.series(bar);
        }
        chart
    }

    /// Creates a new pie chart with the given series and the title.
    fn create_pie_chart(series: series::Pie, title: &str) -> Self {
        let mut chart = Chart::with_defaults(title).with_toolbox(
            Feature::new()
                .save_as_image(SaveAsImage::new())
                .data_view(DataView::new()),
        );
        chart = chart.series(series);
        chart
    }

    /// Renders the chart into an SVG with the theme. If `custom_theme` is `None`, [`Theme::Default`] is used.
    fn render_svg(
        &self,
        width: u32,
        height: u32,
        custom_theme: Option<&str>,
    ) -> Result<String, ChartSettingError> {
        Ok(charming::ImageRenderer::new(width, height)
            .theme(load_theme(custom_theme))
            .render(self)?)
    }

    /// Renders the chart into an HTTP with the theme. If `custom_theme` is `None`, [`Theme::Default`] is used.
    fn render_html(
        &self,
        width: u64,
        height: u64,
        custom_theme: Option<&str>,
    ) -> Result<String, ChartSettingError> {
        let mut html = charming::HtmlRenderer::new("Chart", width, height)
            //.theme(load_theme(custom_theme)) // Broken in charming 0.6.0
            .render(self)?;

        // Workaround to inject theme into HTML due to upstream bug.
        html = html.replace("'chart'))", "'chart'), echartsTheme)");
        let theme_name = match custom_theme {
            Some(_) => "'custom'",
            None => "''",
        };

        // Inject setColorTheme() from the dashboard and add a fallback for viewing the individual HTML chart files
        html = html.replace(
            r#"<script type="text/javascript">"#,
            &format!(
                "<script type=\"text/javascript\">
            const echartsTheme = typeof setColorTheme === 'function' ? setColorTheme({theme_name}) : {theme_name};"
            ),
        );

        // If no theme is specified, return here
        let Some(theme) = custom_theme else {
            return Ok(html);
        };

        // Inject the loader with the theme file contents into the HTML
        let loader = include_str!("charming_theme_loader.js").replace("JSON", theme);
        html = html.replace(r"'custom';", &format!("'custom';\n{loader}"));

        Ok(html)
    }
}

/// Creates a custom theme with the JSON in `custom_theme`.
fn load_theme(custom_theme: Option<&str>) -> Theme {
    custom_theme.map_or(Theme::Default, |theme| {
        let theme_loader = include_str!("charming_theme_loader.js").replace("JSON", theme);
        Theme::Custom("custom", Box::leak(theme_loader.into_boxed_str()))
    })
}
