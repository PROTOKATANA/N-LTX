//! Command ids of the basic layer and the TeX that registers them.

use crate::tex::Meaning;

use super::super::Engine;

pub const BASE: u16 = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::engine::basic) enum B {
    Process,
    ExtractX,
    ExtractY,
    MoveTo,
    LineTo,
    CurveTo,
    QuadraticCurveTo,
    Close,
    Arc,
    ArcAxes,
    Ellipse,
    Circle,
    Rectangle,
    RectangleCorners,
    Grid,
    Parabola,
    Sine,
    Cosine,
    UsePath,
    UsePathQStroke,
    UsePathQFill,
    UsePathQFillStroke,
    UsePathQClip,
    LineWidth,
    Dash,
    StrokeColor,
    FillColor,
    SetColor,
    StrokeOpacity,
    FillOpacity,
    ButtCap,
    RoundCap,
    RectCap,
    MiterJoin,
    RoundJoin,
    BevelJoin,
    MiterLimit,
    EoRule,
    NonzeroRule,
    Arrows,
    ArrowsStart,
    ArrowsEnd,
    ShortenStart,
    ShortenEnd,
    CornersArced,
    Shift,
    XShift,
    YShift,
    Scale,
    XScale,
    YScale,
    XSlant,
    YSlant,
    Rotate,
    Cm,
    Reset,
    ResetNonTranslations,
    Scope,
    EndScope,
    Picture,
    EndPicture,
    Text,
    Node,
    MultipartNode,
    Coordinate,
    XVec,
    YVec,
    ZVec,
    /// `\pgfplothandlerlineto`… (the handler's name in [`PLOT_HANDLERS`])
    PlotHandler,
    PlotHandlerMark,
    PlotHandlerMarkListed,
    StreamStart,
    StreamPoint,
    StreamEnd,
    PlotTension,
    PlotHandlerRecord,
    PlotFunction,
    FirstLineTo,
    FirstMoveTo,
    UsePlotMark,
    PlotMarkSize,
    DeclarePlotMark,
    Triangle,
    ArrowFrame,
    LineAtTime,
    CurveAtTime,
    ArcAxesAtTime,
    Invert,
    Adjustments,
    LowLevel,
    LowLevelObj,
    LowLevelSync,
    NonlinearFlatness,
    QMoveTo,
    QLineTo,
    QCurveTo,
    QCircle,
    ArcTo,
    ArcToPrecomputed,
    DeclareFunctional,
    ShadeColorToRgb,
    GetPath,
    SetPath,
    IntersectionOfPaths,
    SetKeys,
    DecToBase,
    ToDigitList,
    InnerLineWidth,
    InnerStrokeColor,
    Baseline,
    BaselineLater,
    BaselineNow,
    Svg,
    DeclarePlotHandler,
    UsePlotHandler,
    SysAnimate,
    GetLastXY,
}

pub(in crate::engine::basic) const COMMANDS: &[(&str, B)] = &[
    ("pgf@process", B::Process),
    ("pgfextractx", B::ExtractX),
    ("pgfextracty", B::ExtractY),
    ("pgfpathmoveto", B::MoveTo),
    ("pgfclosepath", B::Close),
    ("pgfpathlineto", B::LineTo),
    ("pgfpathcurveto", B::CurveTo),
    ("pgfpathquadraticcurveto", B::QuadraticCurveTo),
    ("pgfpathclose", B::Close),
    ("pgfpatharc", B::Arc),
    ("pgfpatharcaxes", B::ArcAxes),
    ("pgfpathellipse", B::Ellipse),
    ("pgfpathcircle", B::Circle),
    ("pgfpathrectangle", B::Rectangle),
    ("pgfpathrectanglecorners", B::RectangleCorners),
    ("pgfpathgrid", B::Grid),
    ("pgfpathparabola", B::Parabola),
    ("pgfpathsine", B::Sine),
    ("pgfpathcosine", B::Cosine),
    ("pgfusepath", B::UsePath),
    ("pgfusepathqstroke", B::UsePathQStroke),
    ("pgfusepathqfill", B::UsePathQFill),
    ("pgfusepathqfillstroke", B::UsePathQFillStroke),
    ("pgfusepathqclip", B::UsePathQClip),
    ("pgfsetlinewidth", B::LineWidth),
    ("pgfsetdash", B::Dash),
    ("pgfsetstrokecolor", B::StrokeColor),
    ("pgfsetfillcolor", B::FillColor),
    ("pgfsetcolor", B::SetColor),
    ("pgfsetstrokeopacity", B::StrokeOpacity),
    ("pgfsetfillopacity", B::FillOpacity),
    ("pgfsetbuttcap", B::ButtCap),
    ("pgfsetroundcap", B::RoundCap),
    ("pgfsetrectcap", B::RectCap),
    ("pgfsetmiterjoin", B::MiterJoin),
    ("pgfsetroundjoin", B::RoundJoin),
    ("pgfsetbeveljoin", B::BevelJoin),
    ("pgfsetmiterlimit", B::MiterLimit),
    ("pgfseteorule", B::EoRule),
    ("pgfsetnonzerorule", B::NonzeroRule),
    ("pgfsetarrows", B::Arrows),
    ("pgfsetarrowsstart", B::ArrowsStart),
    ("pgfsetarrowsend", B::ArrowsEnd),
    ("pgfsetshortenstart", B::ShortenStart),
    ("pgfsetshortenend", B::ShortenEnd),
    ("pgfsetcornersarced", B::CornersArced),
    ("pgftransformshift", B::Shift),
    ("pgftransformxshift", B::XShift),
    ("pgftransformyshift", B::YShift),
    ("pgftransformscale", B::Scale),
    ("pgftransformxscale", B::XScale),
    ("pgftransformyscale", B::YScale),
    ("pgftransformxslant", B::XSlant),
    ("pgftransformyslant", B::YSlant),
    ("pgftransformrotate", B::Rotate),
    ("pgftransformcm", B::Cm),
    ("pgftransformreset", B::Reset),
    ("pgftransformresetnontranslations", B::ResetNonTranslations),
    ("pgfscope", B::Scope),
    ("endpgfscope", B::EndScope),
    ("pgfpicture", B::Picture),
    ("endpgfpicture", B::EndPicture),
    ("pgftext", B::Text),
    ("pgfnode", B::Node),
    ("pgfmultipartnode", B::MultipartNode),
    ("pgfcoordinate", B::Coordinate),
    ("pgfsetxvec", B::XVec),
    ("pgfsetyvec", B::YVec),
    ("pgfsetzvec", B::ZVec),
    ("pgfplothandlermark", B::PlotHandlerMark),
    ("pgfplothandlermarklisted", B::PlotHandlerMarkListed),
    ("pgfplotstreamstart", B::StreamStart),
    ("pgfplotstreampoint", B::StreamPoint),
    ("pgfplotstreamend", B::StreamEnd),
    ("pgfsetplottension", B::PlotTension),
    ("pgfplothandlerlineto", B::PlotHandler),
    ("pgfplothandlerpolygon", B::PlotHandler),
    ("pgfplothandlercurveto", B::PlotHandler),
    ("pgfplothandlerclosedcurve", B::PlotHandler),
    ("pgfplothandlerdiscard", B::PlotHandler),
    ("pgfplothandlerxcomb", B::PlotHandler),
    ("pgfplothandlerycomb", B::PlotHandler),
    ("pgfplothandlerpolarcomb", B::PlotHandler),
    ("pgfplothandlerconstantlineto", B::PlotHandler),
    ("pgfplothandlerconstantlinetomarkright", B::PlotHandler),
    ("pgfplothandlerconstantlinetomarkmid", B::PlotHandler),
    ("pgfplothandlerjumpmarkleft", B::PlotHandler),
    ("pgfplothandlerjumpmarkright", B::PlotHandler),
    ("pgfplothandlerjumpmarkmid", B::PlotHandler),
    ("pgfplothandlerxbar", B::PlotHandler),
    ("pgfplothandlerybar", B::PlotHandler),
    ("pgfplothandlerxbarinterval", B::PlotHandler),
    ("pgfplothandlerybarinterval", B::PlotHandler),
    ("pgfplothandlergaplineto", B::PlotHandler),
    ("pgfplothandlergapcycle", B::PlotHandler),
    ("pgfplothandlerrecord", B::PlotHandlerRecord),
    ("pgfplotfunction", B::PlotFunction),
    ("pgfsetlinetofirstplotpoint", B::FirstLineTo),
    ("pgfsetmovetofirstplotpoint", B::FirstMoveTo),
    ("pgfuseplotmark", B::UsePlotMark),
    ("pgfsetplotmarksize", B::PlotMarkSize),
    ("pgfdeclareplotmark", B::DeclarePlotMark),
    ("pgftransformtriangle", B::Triangle),
    ("pgftransformarrow", B::ArrowFrame),
    ("pgftransformlineattime", B::LineAtTime),
    ("pgftransformcurveattime", B::CurveAtTime),
    ("pgftransformarcaxesattime", B::ArcAxesAtTime),
    ("pgftransforminvert", B::Invert),
    ("pgftransformationadjustments", B::Adjustments),
    ("pgflowlevel", B::LowLevel),
    ("pgflowlevelobj", B::LowLevelObj),
    ("pgflowlevelsynccm", B::LowLevelSync),
    ("pgfsettransformnonlinearflatness", B::NonlinearFlatness),
    ("pgfpathqmoveto", B::QMoveTo),
    ("pgfpathqlineto", B::QLineTo),
    ("pgfpathqcurveto", B::QCurveTo),
    ("pgfpathqcircle", B::QCircle),
    ("pgfpatharcto", B::ArcTo),
    ("pgfpatharctoprecomputed", B::ArcToPrecomputed),
    ("pgfdeclarefunctionalshading", B::DeclareFunctional),
    ("pgfshadecolortorgb", B::ShadeColorToRgb),
    ("pgfgetpath", B::GetPath),
    ("pgfsetpath", B::SetPath),
    ("pgfintersectionofpaths", B::IntersectionOfPaths),
    ("setkeys", B::SetKeys),
    ("pgfmathdectobase", B::DecToBase),
    ("pgfmathtodigitlist", B::ToDigitList),
    ("pgfsetinnerlinewidth", B::InnerLineWidth),
    ("pgfsetinnerstrokecolor", B::InnerStrokeColor),
    ("pgfsetbaseline", B::Baseline),
    ("pgfsetbaselinepointlater", B::BaselineLater),
    ("pgfsetbaselinepointnow", B::BaselineNow),
    ("pgfpathsvg", B::Svg),
    ("pgfdeclareplothandler", B::DeclarePlotHandler),
    ("tikra@plothandler", B::UsePlotHandler),
    ("tikra@sysanimate", B::SysAnimate),
    ("pgfgetlastxy", B::GetLastXY),
];

/// The basic layer's definitions made in TeX: the switches of the `…attime` transformations, the
/// transformation adjustments before they are computed, and `pgflowlevelscope`. The animations of the
/// basic layer take their arguments and do nothing: a still picture shows what they animate in its
/// base state. Those of the system layer do too, unless a snapshot is set (`\pgfsysanimsnapshot`):
/// their keys make a timeline, which `\pgfsysanimate` turns into the attribute's value at the
/// snapshot time, given to the object the id refers to.
pub const DEFINITIONS: &str = r"
% pgf's scratch counts, which documents borrow
\newcount\c@pgf@counta \newcount\c@pgf@countb \newcount\c@pgf@countc \newcount\c@pgf@countd
\newif\ifpgfslopedattime \newif\ifpgfallowupsidedownattime \newif\ifpgfresetnontranslationattime
\def\pgfhorizontaltransformationadjustment{1}\def\pgfverticaltransformationadjustment{1}
\newenvironment{pgflowlevelscope}[1]{\pgfscope\pgflowlevel{#1}}{\endpgfscope}
\def\pgfidrefnextuse#1#2{\def#1{#2}}\def\pgfidrefprevuse#1#2{\def#1{#2}}
\def\pgfsetblendmode#1{\tikzset{blend mode={#1}}}
\def\tikzdeclarecoordinatesystem#1#2{\expandafter\def\csname tikra@cs@#1\endcsname##1{#2}}
\def\define@key#1#2{\expandafter\def\csname KV@#1@#2\endcsname##1}
\def\pgfmathsetbasenumberlength#1{\def\pgfmathbasenumberlength{#1}}\def\pgfmathbasenumberlength{1}
% pgfmath's commands of one function: \pgfmathresult is its value
\def\pgfmathsin#1{\pgfmathparse{sin(#1)}}
\def\pgfmathcos#1{\pgfmathparse{cos(#1)}}
\def\pgfmathtan#1{\pgfmathparse{tan(#1)}}
\def\pgfmathsqrt#1{\pgfmathparse{sqrt(#1)}}
\def\pgfmathabs#1{\pgfmathparse{abs(#1)}}
\def\pgfmathneg#1{\pgfmathparse{neg(#1)}}
\def\pgfmathround#1{\pgfmathparse{round(#1)}}
\def\pgfmathfloor#1{\pgfmathparse{floor(#1)}}
\def\pgfmathceil#1{\pgfmathparse{ceil(#1)}}
\def\pgfmathexp#1{\pgfmathparse{exp(#1)}}
\def\pgfmathln#1{\pgfmathparse{ln(#1)}}
\def\pgfmathdeg#1{\pgfmathparse{deg(#1)}}
\def\pgfmathrad#1{\pgfmathparse{rad(#1)}}
\def\pgfmathint#1{\pgfmathparse{int(#1)}}
\def\pgfmathfrac#1{\pgfmathparse{frac(#1)}}
\def\pgfmathasin#1{\pgfmathparse{asin(#1)}}
\def\pgfmathacos#1{\pgfmathparse{acos(#1)}}
\def\pgfmathatan#1{\pgfmathparse{atan(#1)}}
\def\pgfmathadd#1#2{\pgfmathparse{(#1)+(#2)}}
\def\pgfmathsubtract#1#2{\pgfmathparse{(#1)-(#2)}}
\def\pgfmathmultiply#1#2{\pgfmathparse{(#1)*(#2)}}
\def\pgfmathdivide#1#2{\pgfmathparse{(#1)/(#2)}}
\def\pgfmathmax#1#2{\pgfmathparse{max(#1,#2)}}
\def\pgfmathmin#1#2{\pgfmathparse{min(#1,#2)}}
\def\pgfmathmod#1#2{\pgfmathparse{mod(#1,#2)}}
\def\pgfmathveclen#1#2{\pgfmathparse{veclen(#1,#2)}}
\def\pgfmathpow#1#2{\pgfmathparse{pow(#1,#2)}}
\def\pgfmathatan2#1#2{\pgfmathparse{atan2(#1,#2)}}
\def\pgfmathreciprocal#1{\pgfmathparse{1/(#1)}}
\def\breakforeach{\gdef\tikra@foreach@break{1}}
\def\pgfanimateattribute#1#2{}\def\pgfsnapshot#1{}\def\pgfsnapshotafter#1{}
\def\pgfsysanimsnapshot#1{\tikzset{make snapshot of={#1}}}\def\pgfsysanimsnapshotafter#1{\tikzset{make snapshot after={#1}}}
\def\tikra@anim@spec{}\def\tikra@anim@whom{}\def\tikra@anim@time{0}
\def\tikra@anim@add#1{\edef\tikra@anim@spec{\tikra@anim@spec#1,}}
\def\tikra@anim@val#1{\tikra@anim@add{\tikra@anim@time={#1}}}
\def\pgfsysanimate#1{\tikra@sysanimate{#1}{\tikra@anim@whom}{\tikra@anim@spec}\def\tikra@anim@spec{}}
\def\pgfsysanimkeywhom#1#2{\edef\tikra@anim@whom{#1}}\def\pgfsysanimkeytime#1#2#3#4#5{\def\tikra@anim@time{#1}}
\def\pgfsysanimkeyfreezeatend{\tikra@anim@add{freeze}}\def\pgfsysanimkeysnapshotstart#1{\tikra@anim@add{begin snapshot=#1}}
\def\pgfsysanimvalscalar#1{\tikra@anim@val{#1}}\def\pgfsysanimvaldimension#1{\tikra@anim@val{#1}}
\def\pgfsysanimkeyrestartalways{}\def\pgfsysanimkeyrestartnever{}\def\pgfsysanimkeyrestartwhennotactive{}
\def\pgfsysanimkeyrepeat#1{}\def\pgfsysanimkeyrepeatindefinite{}\def\pgfsysanimkeyrepeatdur#1{}
\def\pgfsysanimkeyremoveatend{}
\def\pgfsysanimkeybase{\def\tikra@anim@time{base}}\def\pgfsysanimkeyoffset#1#2{}
\def\pgfsysanimkeysyncbegin#1#2#3#4{}\def\pgfsysanimkeysyncend#1#2#3#4{}
\def\pgfsysanimkeyevent#1#2#3#4#5{}\def\pgfsysanimkeyrepeatevent#1#2#3#4#5{}
\def\pgfsysanimkeyaccesskey#1#2#3{}\def\pgfsysanimkeyaccumulate{}\def\pgfsysanimkeynoaccumulate{}
\def\pgfsysanimkeyrotatealong{}\def\pgfsysanimkeynorotatealong{}
\def\pgfsysanimkeymovealong#1{}\def\pgfsysanimkeytipmarkers#1#2{}\def\pgfsysanimkeycanvastransform#1#2{}
\def\pgfsysanimvalcurrent{}\def\pgfsysanimvalnone{}\def\pgfsysanimvaltext#1{}
\def\pgfsysanimvalcolor#1{}\def\pgfsysanimvalcolorrgb#1#2#3{}
\def\pgfsysanimvalcolorcmyk#1#2#3#4{}\def\pgfsysanimvalcolorcmy#1#2#3{}\def\pgfsysanimvalcolorgray#1{}
\def\pgfsysanimvalpath#1{}\def\pgfsysanimvaltranslate#1#2{}\def\pgfsysanimvalscale#1#2{}
\def\pgfsysanimvalviewbox#1#2#3#4{}\def\pgfsysanimvaldash#1#2{}
";

/// The basic layer's plot handlers, by command, as TikZ's `plot` names them.
pub(in crate::engine::basic) const PLOT_HANDLERS: &[(&str, &str)] = &[
    ("pgfplothandlerlineto", "sharp plot"),
    ("pgfplothandlerpolygon", "sharp cycle"),
    ("pgfplothandlercurveto", "smooth"),
    ("pgfplothandlerclosedcurve", "smooth cycle"),
    ("pgfplothandlerdiscard", "discard"),
    ("pgfplothandlerxcomb", "xcomb"),
    ("pgfplothandlerycomb", "ycomb"),
    ("pgfplothandlerpolarcomb", "polar comb"),
    ("pgfplothandlerconstantlineto", "const plot"),
    ("pgfplothandlerconstantlinetomarkright", "const plot mark right"),
    ("pgfplothandlerconstantlinetomarkmid", "const plot mark mid"),
    ("pgfplothandlerjumpmarkleft", "jump mark left"),
    ("pgfplothandlerjumpmarkright", "jump mark right"),
    ("pgfplothandlerjumpmarkmid", "jump mark mid"),
    ("pgfplothandlerxbar", "xbar"),
    ("pgfplothandlerybar", "ybar"),
    ("pgfplothandlerxbarinterval", "xbar interval"),
    ("pgfplothandlerybarinterval", "ybar interval"),
    ("pgfplothandlergaplineto", "gap lineto"),
    ("pgfplothandlergapcycle", "gap cycle"),
];

/// The names a point argument can start with.
pub(in crate::engine::basic) const POINTS: &[&str] = &[
    "pgfpoint",
    "pgfqpoint",
    "pgfpointorigin",
    "pgfpointpolar",
    "pgfqpointpolar",
    "pgfpointxy",
    "pgfqpointxy",
    "pgfpointxyz",
    "pgfqpointxyz",
    "pgfpointpolarxy",
    "pgfpointcylindrical",
    "pgfpointspherical",
    "pgfpointadd",
    "pgfpointdiff",
    "pgfpointscale",
    "pgfqpointscale",
    "pgfpointlineattime",
    "pgfpointlineatdistance",
    "pgfpointcurveattime",
    "pgfpointnormalised",
    "pgfpointanchor",
    "pgfpointshapeborder",
    "pgfpointintersectionoflines",
    "pgfpointtransformed",
    "pgfpointarcaxesattime",
    "pgfpointintersectionsolution",
];

pub fn define(e: &mut Engine) {
    for (i, (name, _)) in COMMANDS.iter().enumerate() {
        e.tex.define(name, Meaning::Native(BASE + i as u16));
    }
    // points are read by the commands that take them; on their own they put the point in
    // `\pgf@x` and `\pgf@y`
    for (i, name) in POINTS.iter().enumerate() {
        e.tex.define(name, Meaning::Native(POINT_BASE + i as u16));
    }
}

/// Where the points' ids start, as statements.
pub(in crate::engine::basic) const POINT_BASE: u16 = 700;

pub fn is_basic(id: u16) -> bool {
    (BASE..BASE + COMMANDS.len() as u16).contains(&id) || (POINT_BASE..POINT_BASE + POINTS.len() as u16).contains(&id)
}
