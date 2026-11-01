//! pgf's basic layer: `\pgfpathmoveto`, `\pgfusepath`, `\pgfpoint…`, `\pgfset…`, `\pgftransform…`,
//! `pgfscope`, `pgfpicture`, `\pgftext`, `\pgfnode`. Points are in the current coordinate system,
//! as pgf keeps them; the current transformation applies when a point goes into a path.

mod catalog;
mod path;
mod picture;
mod points;

pub use catalog::{define, is_basic, BASE, DEFINITIONS};
pub use path::{softpath_of, PgfPath};

use crate::scene::{Segment, Transform};
use crate::tex::{self, Token};
use crate::{Error, PT};

use super::{tikra_tokens, Engine};
use catalog::{B, COMMANDS, PLOT_HANDLERS, POINTS, POINT_BASE};

impl Engine {
    pub fn basic_command(&mut self, id: u16) -> Result<(), Error> {
        if id >= POINT_BASE {
            // a point on its own: into `\pgf@x`, `\pgf@y` (pt), in the current coordinate system
            let Some(name) = POINTS.get((id - POINT_BASE) as usize) else {
                return Ok(());
            };
            self.tex.back(Token::cs(name));
            let p = self.point_here(0)?;
            self.tex.dimens.insert(4050, (p.x / crate::PT * 65536.0).round() as i64);
            self.tex.dimens.insert(4051, (p.y / crate::PT * 65536.0).round() as i64);
            return Ok(());
        }
        let Some(&(_, cmd)) = COMMANDS.get((id - BASE) as usize) else {
            return Ok(());
        };
        let t = self.st.transform;
        match cmd {
            B::Process => {
                // the point's code run, its result left in `\pgf@x`, `\pgf@y`
                let p = self.point_arg()?;
                self.set_pgf_xy(p);
            }
            B::ExtractX | B::ExtractY => self.extract_coord(cmd)?,
            B::MoveTo => {
                let p = self.point_arg()?;
                self.pgf_add(Segment::MoveTo(t.apply(p)));
            }
            B::LineTo => {
                let p = self.point_arg()?;
                self.pgf_add(Segment::LineTo(t.apply(p)));
            }
            B::CurveTo => {
                let a = self.point_arg()?;
                let b = self.point_arg()?;
                let c = self.point_arg()?;
                self.pgf_add(Segment::CurveTo(t.apply(a), t.apply(b), t.apply(c)));
            }
            B::QuadraticCurveTo => {
                let q = self.point_arg()?;
                let p = self.point_arg()?;
                let a = self.last_local();
                self.pgf_add(Segment::CurveTo(t.apply(a.lerp(q, 2.0 / 3.0)), t.apply(p.lerp(q, 2.0 / 3.0)), t.apply(p)));
            }
            B::Close => self.pgf_add(Segment::Close),
            B::Arc | B::ArcAxes => self.path_arc(cmd, t)?,
            B::Ellipse | B::Circle => self.path_ellipse(cmd, t)?,
            B::Rectangle | B::RectangleCorners => self.path_rectangle(cmd, t)?,
            B::Grid => self.path_grid(t)?,
            B::Parabola => self.path_parabola(t)?,
            B::Sine | B::Cosine => self.path_trig(cmd, t)?,
            B::UsePath => {
                let a = self.argument_string()?;
                self.pgf_use(&a)?;
            }
            B::UsePathQStroke => self.pgf_use("stroke")?,
            B::UsePathQFill => self.pgf_use("fill")?,
            B::UsePathQFillStroke => self.pgf_use("fill,stroke")?,
            B::UsePathQClip => self.pgf_use("clip")?,
            B::LineWidth => {
                let v = self.argument()?;
                let w = self.length_of(v)?;
                self.process_keys(tikra_tokens(&format!("line width={}pt", w / PT)), Some("/tikz"))?;
            }
            B::Dash => self.set_dash()?,
            B::StrokeColor | B::FillColor | B::SetColor => self.set_colors(cmd)?,
            B::StrokeOpacity | B::FillOpacity => {
                let v = self.argument()?;
                let o = self.number_of(v)?;
                if cmd == B::StrokeOpacity {
                    self.st.paint.draw_opacity = o;
                } else {
                    self.st.paint.fill_opacity = o;
                }
            }
            B::ButtCap => self.st.pen.cap = crate::canvas::Cap::Butt,
            B::RoundCap => self.st.pen.cap = crate::canvas::Cap::Round,
            B::RectCap => self.st.pen.cap = crate::canvas::Cap::Square,
            B::MiterJoin => self.st.pen.join = crate::canvas::Join::Miter,
            B::RoundJoin => self.st.pen.join = crate::canvas::Join::Round,
            B::BevelJoin => self.st.pen.join = crate::canvas::Join::Bevel,
            B::MiterLimit => {
                let v = self.argument()?;
                self.st.pen.miter_limit = self.number_of(v)?;
            }
            B::EoRule => self.st.paint.even_odd = true,
            B::NonzeroRule => self.st.paint.even_odd = false,
            B::Arrows | B::ArrowsStart | B::ArrowsEnd => {
                let s = self.argument_string()?;
                match cmd {
                    B::Arrows => {
                        if let Some((a, b)) = super::arrows::parse_arrows(&s, &self.st.pen.tips) {
                            self.st.pen.start_arrow = a;
                            self.st.pen.end_arrow = b;
                        }
                    }
                    B::ArrowsStart => self.st.pen.start_arrow = Some(s).filter(|x| !x.is_empty()),
                    _ => self.st.pen.end_arrow = Some(s).filter(|x| !x.is_empty()),
                }
            }
            B::ShortenStart | B::ShortenEnd => {
                let v = self.argument()?;
                let d = self.length_of(v)?;
                if cmd == B::ShortenStart {
                    self.st.pen.shorten_start = d;
                } else {
                    self.st.pen.shorten_end = d;
                }
            }
            B::CornersArced => {
                let p = self.point_arg()?;
                // the corner's arc starts p.x before it and ends p.y after it
                (self.st.pen.rounded, self.st.pen.rounded_out) = (p.x, p.y);
            }
            B::Shift => {
                let p = self.point_arg()?;
                self.st.transform = t.compose(&Transform::translation(p.x, p.y));
            }
            B::XShift | B::YShift => {
                let v = self.argument()?;
                let d = self.length_of(v)?;
                let s = if cmd == B::XShift { Transform::translation(d, 0.0) } else { Transform::translation(0.0, d) };
                self.st.transform = t.compose(&s);
            }
            B::Scale | B::XScale | B::YScale => {
                let v = self.argument()?;
                let f = self.number_of(v)?;
                let s = match cmd {
                    B::Scale => Transform::scaling(f, f),
                    B::XScale => Transform::scaling(f, 1.0),
                    _ => Transform::scaling(1.0, f),
                };
                self.st.transform = t.compose(&s);
            }
            B::XSlant | B::YSlant => {
                let v = self.argument()?;
                let f = self.number_of(v)?;
                let s = if cmd == B::XSlant { Transform { c: f, ..Transform::default() } } else { Transform { b: f, ..Transform::default() } };
                self.st.transform = t.compose(&s);
            }
            B::Rotate => {
                let v = self.argument()?;
                let a = self.number_of(v)?;
                self.st.transform = t.compose(&Transform::rotation(a));
            }
            B::Cm => self.transform_cm(t)?,
            B::Reset => self.st.transform = Transform::default(),
            B::ResetNonTranslations => self.st.transform = Transform { a: 1.0, b: 0.0, c: 0.0, d: 1.0, ..t },
            B::Scope => {
                self.begin_group();
                self.begin_scope();
            }
            B::EndScope => {
                self.end_scope();
                self.end_group();
            }
            B::Picture => self.basic_picture(true)?,
            B::EndPicture => self.basic_picture(false)?,
            B::Text => self.place_text(t)?,
            B::Node | B::MultipartNode => self.pgf_node(cmd, t)?,
            B::Coordinate => self.place_coordinate(t)?,
            B::XVec | B::YVec | B::ZVec => {
                let p = self.point_arg()?;
                match cmd {
                    B::XVec => self.st.x = p,
                    B::YVec => self.st.y = p,
                    _ => self.st.z = p,
                }
            }
            B::PlotHandler => {
                let name = COMMANDS[(id - BASE) as usize].0;
                let handler = PLOT_HANDLERS.iter().find(|(c, _)| *c == name).map_or("sharp plot", |(_, h)| h);
                self.st.plot.basic_plot = (handler.to_string(), Vec::new());
            }
            B::PlotHandlerMark => {
                let code = self.argument()?;
                self.st.plot.basic_plot = ("mark".into(), code);
            }
            B::PlotHandlerMarkListed => {
                // `{code}{1,3}`: a mark on the listed points only (counted from 1)
                let code = self.argument()?;
                let list = self.argument_string()?;
                self.st.plot.basic_plot = (format!("mark listed:{list}"), code);
            }
            B::PlotTension => {
                let v = self.argument()?;
                self.st.plot.tension = self.number_of(v)?;
            }
            B::StreamStart => self.pgf.plot_stream.clear(),
            B::StreamPoint => {
                let p = self.point_arg()?;
                self.pgf.plot_stream.push(t.apply(p));
            }
            B::StreamEnd => self.plot_stream_end()?,
            B::PlotHandlerRecord => {
                let m = self.argument()?;
                self.st.plot.basic_plot = ("record".into(), m);
            }
            B::FirstLineTo => self.st.plot.plot_first_lineto = true,
            B::FirstMoveTo => self.st.plot.plot_first_lineto = false,
            B::PlotFunction => self.plot_function(t)?,
            B::UsePlotMark => {
                let name = self.argument_string()?;
                self.use_plot_mark(name.trim())?;
            }
            B::PlotMarkSize => {
                let v = self.argument()?;
                self.st.plot.mark_size = self.length_of(v)?;
            }
            B::DeclarePlotMark => {
                let name = self.argument_string()?;
                let code = self.argument()?;
                self.declared.plot_marks_declared.insert(name.trim().to_string(), code);
            }
            B::Triangle => self.transform_triangle(t)?,
            B::ArrowFrame => self.transform_arrow(t)?,
            B::LineAtTime => {
                let time = self.argument()?;
                let time = self.number_of(time)?;
                let p = self.point_arg()?;
                let q = self.point_arg()?;
                self.transform_at_time(p.lerp(q, time), q.minus(p));
            }
            B::CurveAtTime => self.transform_curve()?,
            B::ArcAxesAtTime => {
                let (at, tangent) = self.arc_axes_at_time()?;
                self.transform_at_time(at, tangent);
            }
            B::Invert => self.st.transform = t.inverse(),
            B::Adjustments => self.write_adjustments(t)?,
            B::LowLevel => {
                let code = self.argument()?;
                self.low_level(code)?;
            }
            B::LowLevelObj => self.low_level_object()?,
            B::LowLevelSync => {
                // the coordinate transformation moves to the canvas, line widths and all
                self.open_in_scope(crate::canvas::Item::Transform { transform: t });
                self.st.transform = Transform::default();
            }
            B::NonlinearFlatness => {
                let _ = self.argument()?;
            }
            B::QMoveTo | B::QLineTo | B::QCurveTo | B::QCircle => {
                // the quick commands take canvas lengths, untransformed; a path of them only does
                // not make the picture larger
                let quick = self.pgf.pgf_path.segments.is_empty() || self.pgf.pgf_path.quick;
                self.quick_path(cmd)?;
                self.pgf.pgf_path.quick = quick;
            }
            B::ArcTo => self.path_svg_arc(t)?,
            B::DeclareFunctional => self.declare_functional_shading()?,
            // the picture's baseline: a height, a point's (evaluated now or when the picture ends)
            B::DeclarePlotHandler => self.declare_plot_handler()?,
            B::UsePlotHandler => {
                let name = self.argument_string()?;
                let args = self.argument()?;
                self.st.plot.basic_plot = (format!("custom:{name}"), args);
            }
            B::SysAnimate => {
                let attribute = self.argument_string()?;
                let whom = self.argument_string()?;
                // the timeline the keys made (a macro)
                let spec = self.argument_string()?;
                self.sys_animate(attribute.trim(), whom.trim(), tikra_tokens(&spec))?;
            }
            B::Svg => {
                let data = self.argument_string()?;
                for s in super::path::svg_path(self, &data, self.pgf.pgf_path.last) {
                    self.pgf_add(s);
                }
            }
            B::Baseline => {
                let v = self.argument_string()?;
                self.st.baseline = Some(v);
            }
            B::BaselineLater => {
                let p = self.argument()?;
                self.st.baseline = Some(format!("tikra@point:{}", tex::tokens_to_string(&p)));
            }
            B::BaselineNow => {
                let p = self.point_arg()?;
                self.st.baseline = Some(format!("{}pt", t.apply(p).y / PT));
            }
            B::InnerLineWidth => {
                let v = self.argument()?;
                self.st.pen.inner_line.0 = self.length_of(v)?;
            }
            B::InnerStrokeColor => {
                let c = self.argument_string()?;
                match self.color(&c) {
                    Some(c) => self.st.pen.inner_line.1 = Some(c),
                    None => self.warn(format!("unknown color '{c}'")),
                }
            }
            B::DecToBase => self.dec_to_base()?,
            B::GetLastXY => self.get_last_xy()?,
            B::ToDigitList => self.write_digit_list()?,
            B::SetKeys => self.set_key_family()?,
            B::GetPath | B::SetPath => self.recall_path(cmd)?,
            B::IntersectionOfPaths => self.intersect_paths()?,
            B::ShadeColorToRgb => self.shade_color_to_rgb()?,
            B::ArcToPrecomputed => self.path_arc_precomputed(t)?,
        }
        Ok(())
    }
}
