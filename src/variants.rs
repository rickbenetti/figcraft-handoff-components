//! Variants of a component set: the properties their names spell ("Size=Small, State=Enabled"), the
//! default variant, and the rows of each axis (one property changed at a time). Pure logic.

/// "Property=Option" pairs, in order.
pub type Pairs = Vec<(String, String)>;

/// The pairs a variant's name spells: "A=B, C=D".
pub fn parse_name(name: &str) -> Pairs {
    name.split(',').filter_map(|p| p.split_once('=')).map(|(k, v)| (k.trim().to_string(), v.trim().to_string())).collect()
}

/// The name for pairs: "A=B, C=D".
pub fn join(pairs: &Pairs) -> String {
    pairs.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join(", ")
}

/// One variant of a set: its id, name and size.
#[derive(Clone, Debug, PartialEq)]
pub struct Variant {
    pub id: u64,
    pub name: String,
    pub w: f64,
    pub h: f64,
}

/// A component set: its variants, the properties their names spell (in order) and each one's options
/// (in order of appearance).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SetInfo {
    pub id: u64,
    pub name: String,
    pub variants: Vec<Variant>,
    pub props: Vec<(String, Vec<String>)>,
}

impl SetInfo {
    pub fn new(id: u64, name: String, variants: Vec<Variant>) -> SetInfo {
        let mut props: Vec<(String, Vec<String>)> = Vec::new();
        for v in &variants {
            for (k, o) in parse_name(&v.name) {
                match props.iter_mut().find(|(p, _)| *p == k) {
                    Some((_, opts)) => {
                        if !opts.contains(&o) {
                            opts.push(o);
                        }
                    }
                    None => props.push((k, vec![o])),
                }
            }
        }
        SetInfo { id, name, variants, props }
    }

    /// The first variant's pairs (the default when none is saved).
    pub fn first_pairs(&self) -> Pairs {
        self.variants.first().map(|v| parse_name(&v.name)).unwrap_or_default()
    }

    /// The variant spelled by `pairs` (order doesn't matter).
    pub fn find(&self, pairs: &Pairs) -> Option<&Variant> {
        self.variants.iter().find(|v| {
            let p = parse_name(&v.name);
            p.len() == pairs.len() && pairs.iter().all(|(k, o)| p.iter().any(|(pk, po)| pk == k && po == o))
        })
    }
}

/// The full variant the pairs `typed` point at: what `typed` says, the rest from the first variant. None when a
/// property or an option is unknown, or when no variant has that combination.
pub fn complete(info: &SetInfo, typed: &Pairs) -> Option<Pairs> {
    for (k, v) in typed {
        let (_, opts) = info.props.iter().find(|(p, _)| p == k)?;
        if !opts.contains(v) {
            return None;
        }
    }
    let mut out = info.first_pairs();
    for (k, v) in typed {
        if let Some(slot) = out.iter_mut().find(|(p, _)| p == k) {
            slot.1 = v.clone();
        }
    }
    info.find(&out).is_some().then_some(out)
}

/// One option of an axis.
#[derive(Clone, Debug, PartialEq)]
pub struct AxisOption {
    pub option: String,
    pub name: String,
    /// The variant with that option (and the default's other properties), when it exists.
    pub variant: Option<Variant>,
    pub is_default: bool,
}

/// An axis: a property and its options, the others held at the default.
#[derive(Clone, Debug, PartialEq)]
pub struct Axis {
    pub prop: String,
    pub options: Vec<AxisOption>,
}

/// The axes of a set, in the order of its properties; `order` gives each property's options
/// (display order), else those of the names.
pub fn axes(info: &SetInfo, default: &Pairs, order: &[(String, Vec<String>)]) -> Vec<Axis> {
    let mut out = Vec::new();
    for (prop, names_opts) in &info.props {
        let opts = order.iter().find(|(p, _)| p == prop).map_or(names_opts.clone(), |(_, o)| o.clone());
        let current = default.iter().find(|(p, _)| p == prop).map(|(_, v)| v.clone());
        let options = opts
            .into_iter()
            .map(|option| {
                let mut pairs = default.clone();
                if let Some(slot) = pairs.iter_mut().find(|(p, _)| p == prop) {
                    slot.1 = option.clone();
                }
                let variant = info.find(&pairs).cloned();
                AxisOption { name: join(&pairs), is_default: current.as_deref() == Some(option.as_str()), variant, option }
            })
            .collect();
        out.push(Axis { prop: prop.clone(), options });
    }
    out
}

/// How many variants of `axes` fit in `limit` rows, and how many defined ones are left out.
pub fn within_limit(axes: &[Axis], limit: usize) -> (usize, usize) {
    let defined = axes.iter().flat_map(|a| &a.options).filter(|o| o.variant.is_some()).count();
    (defined.min(limit), defined.saturating_sub(limit))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn button() -> SetInfo {
        let mut vs = Vec::new();
        let mut id = 1;
        for size in ["Small", "Medium", "Large"] {
            for style in ["Compact", "Rounded"] {
                for state in ["Enabled", "Hovered"] {
                    // "Large, Compact, Hovered" doesn't exist.
                    if size == "Large" && style == "Compact" && state == "Hovered" {
                        continue;
                    }
                    vs.push(Variant { id, name: format!("Size={size}, Style={style}, State={state}"), w: 100.0, h: 40.0 });
                    id += 1;
                }
            }
        }
        SetInfo::new(100, "Button".into(), vs)
    }

    #[test]
    fn names_spell_pairs() {
        assert_eq!(parse_name("Size=Small, State=Enabled"), [("Size".to_string(), "Small".to_string()), ("State".to_string(), "Enabled".to_string())]);
        assert_eq!(join(&parse_name("A=B,C = D")), "A=B, C=D");
        assert!(parse_name("Button").is_empty());
    }

    #[test]
    fn props_and_options_follow_the_names() {
        let s = button();
        let names: Vec<&str> = s.props.iter().map(|(p, _)| p.as_str()).collect();
        assert_eq!(names, ["Size", "Style", "State"]);
        assert_eq!(s.props[0].1, ["Small", "Medium", "Large"]);
    }

    #[test]
    fn a_partial_default_takes_the_rest_from_the_first_variant() {
        let s = button();
        let d = complete(&s, &pairs("State=Hovered")).unwrap();
        assert_eq!(join(&d), "Size=Small, Style=Compact, State=Hovered");
        let d = complete(&s, &pairs("Style=Rounded, Size=Medium")).unwrap();
        assert_eq!(join(&d), "Size=Medium, Style=Rounded, State=Enabled");
    }

    /// "Size=Small, State=Hover" as pairs.
    fn pairs(text: &str) -> Pairs {
        text.split(',').filter_map(|p| p.split_once('=')).map(|(k, v)| (k.trim().to_string(), v.trim().to_string())).collect()
    }

    #[test]
    fn a_property_an_option_or_a_combination_that_does_not_exist_gives_nothing() {
        let s = button();
        assert_eq!(complete(&s, &pairs("Colour=Red")), None);
        assert_eq!(complete(&s, &pairs("Size=Huge")), None);
        // Valid pairs, but no such variant.
        assert_eq!(complete(&s, &pairs("Size=Large, Style=Compact, State=Hovered")), None);
    }

    #[test]
    fn axes_change_one_property_at_a_time_and_skip_missing_combinations() {
        let s = button();
        let d = parse_name("Size=Large, Style=Compact, State=Enabled");
        let a = axes(&s, &d, &[]);
        assert_eq!(a.len(), 3);
        // State axis: Hovered doesn't exist for Large + Compact.
        let state = &a[2];
        assert_eq!(state.prop, "State");
        assert!(state.options[0].is_default && state.options[0].variant.is_some());
        assert_eq!(state.options[1].name, "Size=Large, Style=Compact, State=Hovered");
        assert!(state.options[1].variant.is_none());
        // Size axis: all three exist, Large is the default.
        assert_eq!(a[0].options.iter().filter(|o| o.variant.is_some()).count(), 3);
        assert_eq!(a[0].options.iter().position(|o| o.is_default), Some(2));
    }

    #[test]
    fn the_row_limit_counts_defined_variants_only() {
        let s = button();
        let a = axes(&s, &parse_name("Size=Small, Style=Compact, State=Enabled"), &[]);
        assert_eq!(within_limit(&a, 24), (7, 0));
        assert_eq!(within_limit(&a, 5), (5, 2));
    }
}
