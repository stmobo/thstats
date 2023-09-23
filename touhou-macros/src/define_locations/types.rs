use std::mem;
use std::ops::RangeInclusive;

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::Ident;

use super::ast;
use crate::util::{self};

#[derive(Debug, Clone)]
pub struct LocationVariant {
    type_ident: Ident,
    variant_ident: Ident,
    display_name: String,
    needs_spell_id: bool,
    full_path: TokenStream,
}

impl LocationVariant {
    fn new(
        type_ident: Ident,
        variant_ident: Ident,
        display_name: String,
        needs_spell_id: bool,
    ) -> Self {
        let full_path = quote! { #type_ident::#variant_ident };
        Self {
            type_ident,
            variant_ident,
            display_name,
            needs_spell_id,
            full_path,
        }
    }

    pub fn new_start(type_ident: Ident) -> Self {
        Self::new(
            type_ident,
            format_ident!("Start"),
            String::from("Start"),
            false,
        )
    }

    pub fn new_basic_section(
        type_ident: Ident,
        second_half_start: Option<u32>,
        seq: u32,
        override_name: Option<String>,
    ) -> Self {
        let prefix = if second_half_start.is_some() {
            "Second"
        } else {
            "First"
        };

        let seq_num = if let Some(second_half_start) = second_half_start {
            seq.saturating_sub(second_half_start)
        } else {
            seq
        };

        Self::new(
            type_ident,
            format_ident!("{}Half{}", prefix, seq_num + 1),
            override_name.unwrap_or_else(|| format!("{} Half {}", prefix, seq_num + 1)),
            false,
        )
    }

    pub fn new_boss_spells(type_ident: Ident, midboss: bool, seq: u32) -> Self {
        let prefix = if midboss { "Midboss" } else { "Boss" };
        Self::new(
            type_ident,
            format_ident!("{}Spell{}", prefix, seq + 1),
            format!("{} Spell {}", prefix, seq + 1),
            true,
        )
    }

    pub fn new_boss_last_spell(type_ident: Ident, seq: u32, multi_ls: bool) -> Self {
        if multi_ls {
            Self::new(
                type_ident,
                format_ident!("LastSpell{}", seq + 1),
                format!("Last Spell {}", seq + 1),
                true,
            )
        } else {
            Self::new(
                type_ident,
                Ident::new("LastSpell", Span::call_site()),
                String::from("Last Spell"),
                true,
            )
        }
    }

    pub fn new_boss_nonspell(type_ident: Ident, midboss: bool, seq: u32) -> Self {
        let prefix = if midboss { "Midboss" } else { "Boss" };

        Self::new(
            type_ident,
            format_ident!("{}Nonspell{}", prefix, seq + 1),
            format!("{} Nonspell {}", prefix, seq + 1),
            false,
        )
    }

    pub fn type_ident(&self) -> &Ident {
        &self.type_ident
    }

    pub fn variant_ident(&self) -> &Ident {
        &self.variant_ident
    }

    pub fn full_path(&self) -> &TokenStream {
        &self.full_path
    }

    pub fn needs_spell_id(&self) -> bool {
        self.needs_spell_id
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub fn match_pattern(&self, spell_capture_ident: Option<&str>) -> (Option<Ident>, TokenStream) {
        let path = self.full_path();
        if self.needs_spell_id() {
            if let Some(cap_ident) =
                spell_capture_ident.map(|name| Ident::new(name, Span::call_site()))
            {
                (Some(cap_ident.clone()), quote! { #path(#cap_ident) })
            } else {
                (None, quote! { #path(_) })
            }
        } else {
            (None, quote! { #path })
        }
    }
}

fn range_to_tokens(range: &RangeInclusive<u32>) -> TokenStream {
    let start = range.start();
    let end = range.end();
    quote! { #start..=#end }
}

#[derive(Debug, Clone)]
pub enum BossPhase {
    Nonspell {
        variant: LocationVariant,
    },
    Spells {
        variant: LocationVariant,
        spell_ids: RangeInclusive<u32>,
    },
    LastSpell {
        variant: LocationVariant,
        spell_ids: RangeInclusive<u32>,
    },
}

impl BossPhase {
    pub fn variant(&self) -> &LocationVariant {
        match self {
            Self::Nonspell { variant, .. }
            | Self::Spells { variant, .. }
            | Self::LastSpell { variant, .. } => variant,
        }
    }

    pub fn ident(&self) -> &Ident {
        self.variant().variant_ident()
    }

    pub fn match_result(&self) -> &TokenStream {
        self.variant().full_path()
    }
}

#[derive(Debug, Clone)]
pub struct BossFight {
    midboss: bool,
    phases: Vec<BossPhase>,
}

impl BossFight {
    pub fn to_fallback_match_result(&self) -> TokenStream {
        match &self.phases[0] {
            BossPhase::Nonspell { variant } => {
                let full_path = variant.full_path();
                quote! { Some(#full_path) }
            }
            BossPhase::Spells { .. } | BossPhase::LastSpell { .. } => quote! { None },
        }
    }

    pub fn to_resolve_arm(&self, state_ident: &Ident, fallback_result: TokenStream) -> TokenStream {
        let mut prev_was_nonspell = false;
        let mut n_healthbars: u32 =
            self.phases
                .iter()
                .enumerate()
                .fold(0, move |n_healthbars, (idx, phase)| match phase {
                    BossPhase::Nonspell { .. } => {
                        prev_was_nonspell = true;
                        n_healthbars + 1
                    }
                    BossPhase::Spells { .. } => {
                        if mem::replace(&mut prev_was_nonspell, false)
                            || (idx == self.phases.len() - 1)
                        {
                            n_healthbars
                        } else {
                            n_healthbars + 1
                        }
                    }
                    BossPhase::LastSpell { .. } => n_healthbars,
                });

        let spell_ranges: Vec<_> = self
            .phases
            .iter()
            .filter_map(|phase| {
                if let BossPhase::Spells { spell_ids, .. }
                | BossPhase::LastSpell { spell_ids, .. } = phase
                {
                    let result = phase.match_result();
                    let id_pattern = range_to_tokens(spell_ids);
                    Some(quote! {
                        Some((#id_pattern, spell)) => Some(#result(spell))
                    })
                } else {
                    None
                }
            })
            .collect();

        let nonspells: Vec<_> = self
            .phases
            .iter()
            .filter_map(|phase| match phase {
                BossPhase::Nonspell { .. } => {
                    prev_was_nonspell = true;

                    n_healthbars = n_healthbars.saturating_sub(1);
                    let healthbar = n_healthbars as u8;

                    let result = phase.match_result();
                    Some(quote! {
                        #healthbar => Some(#result)
                    })
                }
                BossPhase::Spells { .. } => {
                    if !mem::replace(&mut prev_was_nonspell, false) {
                        n_healthbars = n_healthbars.saturating_sub(1);
                    }

                    None
                }
                BossPhase::LastSpell { .. } => None,
            })
            .collect();

        let nonspell_match = if nonspells.is_empty() {
            quote! { None }
        } else {
            quote! {
                match boss.remaining_lifebars() {
                    #(#nonspells,)*
                    _ => None
                }
            }
        };

        if spell_ranges.is_empty() {
            quote! {
                {
                    use crate::memory::traits::{StageData, BossData, BossLifebars};

                    if let Some(boss) = #state_ident.active_boss() {
                        #nonspell_match
                    } else {
                        #fallback_result
                    }
                },
            }
        } else {
            quote! {
                {
                    use crate::memory::traits::{StageData, BossData, BossLifebars};

                    if let Some(boss) = #state_ident.active_boss() {
                        match boss.active_spell().map(|state| (state.raw_spell_id(), state.spell())) {
                            #(#spell_ranges,)*
                            Some(_) => None,
                            None => #nonspell_match
                        }
                    } else {
                        #fallback_result
                    }
                },
            }
        }
    }
}

#[derive(Debug)]
pub enum FrameSpanIter<'a> {
    Single(Option<&'a LocationVariant>),
    Boss(bool, std::slice::Iter<'a, BossPhase>),
}

impl<'a> Iterator for FrameSpanIter<'a> {
    type Item = &'a LocationVariant;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Single(inner) => inner.take(),
            Self::Boss(midboss, inner) => inner.next().map(|phase| phase.variant()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum FrameSpanType {
    Single(LocationVariant),
    Boss(BossFight),
}

impl FrameSpanType {
    fn to_fallback_match_result(&self) -> TokenStream {
        match self {
            Self::Single(variant) => {
                let path = variant.full_path();
                quote! { Some(#path) }
            }
            Self::Boss(fight) => fight.to_fallback_match_result(),
        }
    }

    fn to_resolve_arm(&self, state_ident: &Ident, fallback_result: TokenStream) -> TokenStream {
        match self {
            Self::Single(variant) => {
                let path = variant.full_path();
                quote! { Some(#path), }
            }
            Self::Boss(fight) => fight.to_resolve_arm(state_ident, fallback_result),
        }
    }

    pub fn iter_variants(&self) -> FrameSpanIter<'_> {
        match self {
            Self::Single(variant) => FrameSpanIter::Single(Some(variant)),
            Self::Boss(fight) => FrameSpanIter::Boss(fight.midboss, fight.phases.iter()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FrameSpan {
    start_frame: u32,
    span_type: FrameSpanType,
}

impl FrameSpan {
    fn to_time_match_arm(
        &self,
        state_ident: &Ident,
        next_span: Option<&FrameSpan>,
        fallback_span: Option<&FrameSpan>,
    ) -> TokenStream {
        let fallback_result = fallback_span
            .or(next_span)
            .map(|span| span.span_type.to_fallback_match_result())
            .unwrap_or_else(|| quote! { None });

        let resolve_arm = self.span_type.to_resolve_arm(state_ident, fallback_result);
        if let Some(end_frame) = next_span.map(|span| span.start_frame - 1) {
            let frames = range_to_tokens(&(self.start_frame..=end_frame));
            quote! {
                #frames => #resolve_arm
            }
        } else {
            quote! { _  => #resolve_arm }
        }
    }

    pub fn iter_variants(&self) -> FrameSpanIter<'_> {
        self.span_type.iter_variants()
    }
}

#[derive(Debug)]
pub struct StageState {
    type_ident: Ident,
    midboss_seq: Option<(u32, u32)>,
    boss_seq: Option<(u32, u32)>,
    second_half_start: Option<u32>,
    stage_seq: u32,
    has_nonspells: bool,
    frame_spans: Vec<FrameSpan>,
}

impl StageState {
    fn new(type_ident: Ident) -> Self {
        Self {
            midboss_seq: None,
            boss_seq: None,
            stage_seq: 0,
            has_nonspells: false,
            second_half_start: None,
            frame_spans: vec![FrameSpan {
                start_frame: 0,
                span_type: FrameSpanType::Single(LocationVariant::new_start(type_ident.clone())),
            }],
            type_ident,
        }
    }

    fn push_stage(
        &mut self,
        frame_number: u32,
        err_span: Span,
        def: &ast::SectionDef,
        name: Option<String>,
    ) -> Result<(), syn::Error> {
        self.frame_spans.push(FrameSpan {
            start_frame: frame_number,
            span_type: FrameSpanType::Single(LocationVariant::new_basic_section(
                self.type_ident.clone(),
                self.second_half_start,
                self.stage_seq,
                name,
            )),
        });

        self.stage_seq += 1;

        Ok(())
    }

    fn push_boss(
        &mut self,
        err_span: Span,
        def: &ast::BossDef,
        frame_number: u32,
        midboss: bool,
    ) -> Result<(), syn::Error> {
        use ast::BossPhaseDef;

        if midboss && self.boss_seq.is_some() {
            return Err(syn::Error::new(
                err_span,
                "cannot define midboss section after boss fight",
            ));
        }

        if midboss && self.second_half_start.is_none() {
            self.second_half_start = Some(self.stage_seq);
        }

        let seq_numbers = if midboss {
            self.midboss_seq.get_or_insert((0, 0))
        } else {
            self.boss_seq.get_or_insert((0, 0))
        };

        let mut phases = Vec::with_capacity(def.phases.len());
        for phase_def in &def.phases {
            match phase_def {
                BossPhaseDef::Nonspell { .. } => {
                    let phase = BossPhase::Nonspell {
                        variant: LocationVariant::new_boss_nonspell(
                            self.type_ident.clone(),
                            midboss,
                            seq_numbers.0,
                        ),
                    };
                    seq_numbers.0 += 1;
                    self.has_nonspells = true;
                    phases.push(phase);
                }
                BossPhaseDef::Spells { range, .. } => {
                    let phase = BossPhase::Spells {
                        variant: LocationVariant::new_boss_spells(
                            self.type_ident.clone(),
                            midboss,
                            seq_numbers.1,
                        ),
                        spell_ids: range.parse_range()?,
                    };
                    seq_numbers.1 += 1;
                    phases.push(phase);
                }
                BossPhaseDef::LastSpell { ranges, .. } => {
                    for (idx, range) in ranges.iter().enumerate() {
                        phases.push(BossPhase::LastSpell {
                            variant: LocationVariant::new_boss_last_spell(
                                self.type_ident.clone(),
                                idx as u32,
                                ranges.len() > 1,
                            ),
                            spell_ids: range.parse_range()?,
                        })
                    }
                }
            };
        }

        self.frame_spans.push(FrameSpan {
            start_frame: frame_number,
            span_type: FrameSpanType::Boss(BossFight { midboss, phases }),
        });

        Ok(())
    }

    pub fn push_ast(
        &mut self,
        frame_number: u32,
        entry: &ast::SectionEntry,
    ) -> Result<(), syn::Error> {
        use ast::SectionDef;

        match &entry.def {
            SectionDef::Basic { name, .. } => self.push_stage(
                frame_number,
                entry.frame_number.span(),
                &entry.def,
                name.as_ref().map(|(_, s)| s.value()),
            ),
            SectionDef::Midboss { def, .. } => {
                self.push_boss(entry.frame_number.span(), def, frame_number, true)
            }
            SectionDef::Boss { def, .. } => {
                self.push_boss(entry.frame_number.span(), def, frame_number, false)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct StageLocations {
    game_type: Ident,
    type_ident: Ident,
    stage_ident: Ident,
    has_nonspells: bool,
    frame_spans: Vec<FrameSpan>,
}

impl StageLocations {
    pub fn from_ast(game_type: Ident, def: &ast::StageDef) -> Result<Self, syn::Error> {
        let type_ident = def
            .override_type_name
            .clone()
            .unwrap_or_else(|| format_ident!("Stage{}", &def.stage_id));
        let mut state = StageState::new(type_ident.clone());
        let mut entries: Vec<(u32, _)> = def
            .sections
            .iter()
            .map(|entry| {
                entry
                    .frame_number
                    .base10_parse()
                    .map(|frame| (frame, entry))
            })
            .collect::<Result<Vec<_>, _>>()?;

        entries.sort_by_key(|kv| kv.0);
        for (frame_number, entry) in entries {
            state.push_ast(frame_number, entry)?;
        }

        Ok(Self {
            game_type,
            type_ident,
            stage_ident: def.stage_id.clone(),
            has_nonspells: state.has_nonspells,
            frame_spans: state.frame_spans,
        })
    }

    pub fn iter_variants(&self) -> impl Iterator<Item = &LocationVariant> + '_ {
        self.frame_spans
            .iter()
            .flat_map(|span| span.iter_variants())
    }

    fn iter_match_patterns<'s>(
        &'s self,
        capture_name: Option<&'s str>,
    ) -> impl Iterator<Item = (&'s LocationVariant, Option<Ident>, TokenStream)> + 's {
        self.iter_variants().map(move |variant| {
            let pattern = variant.match_pattern(capture_name);
            (variant, pattern.0, pattern.1)
        })
    }

    fn resolve_match_arms(&self, state_ident: &Ident) -> TokenStream {
        let mut ret = TokenStream::new();
        let mut fallback_span = None;
        let mut iter = self.frame_spans.iter().peekable();

        while let Some(frame_span) = iter.next() {
            if matches!(frame_span.span_type, FrameSpanType::Single(_)) {
                fallback_span = Some(frame_span);
            }

            let fallback = if matches!(
                frame_span.span_type,
                FrameSpanType::Boss(BossFight { midboss: false, .. })
            ) {
                None
            } else {
                fallback_span
            };

            ret.extend(frame_span.to_time_match_arm(state_ident, iter.peek().copied(), fallback));
        }

        ret
    }

    fn define_enum(&self) -> TokenStream {
        let variants = self.iter_variants().map(|variant| {
            let name = variant.variant_ident();
            let game = &self.game_type;
            if variant.needs_spell_id() {
                quote! { #name(crate::types::SpellCard<crate::#game>) }
            } else {
                quote! { #name }
            }
        });

        let name_map = self.iter_match_patterns(None).map(|(variant, _, pattern)| {
            let name = variant.display_name();
            quote! { #pattern => #name }
        });

        let display_map = self.iter_match_patterns(Some("spell")).map(|(variant, cap_ident, pattern)| {
            let name = variant.display_name();
            if variant.needs_spell_id() {
                quote! {
                    #pattern => write!(f, "{} (#{:03} {})", #name, #cap_ident.id(), #cap_ident.name())
                }
            } else {
                quote! { #pattern => f.write_str(#name) }
            }
        });

        let match_spell_map =
            self.iter_match_patterns(Some("spell"))
                .filter_map(|(_, spell_ident, pattern)| {
                    spell_ident.map(|ident| quote! { #pattern => Some(#ident.clone()) })
                });

        let state_ident = format_ident!("state");
        let resolve_match_arms = self.resolve_match_arms(&state_ident);

        let last_variant_pattern = self
            .iter_variants()
            .last()
            .map(|variant| variant.match_pattern(None).1)
            .unwrap();

        let type_name = &self.type_ident;
        let game = &self.game_type;

        let resolve_bounds = if self.has_nonspells {
            quote! {
                T: crate::memory::traits::StageData<#game>,
                T::BossState: crate::memory::traits::BossLifebars
            }
        } else {
            quote! {
                T: crate::memory::traits::StageData<#game>
            }
        };

        quote! {
            #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
            #[serde(tag = "type", content = "spell", rename_all = "snake_case")]
            pub enum #type_name {
                #(#variants),*
            }

            #[automatically_derived]
            impl #type_name {
                pub fn resolve<T>(#state_ident: &T) -> Option<Self>
                    where #resolve_bounds
                {
                    use crate::memory::traits::*;
                    match #state_ident.ecl_time() {
                        #resolve_match_arms
                    }
                }

                pub fn name(&self) -> &'static str {
                    match self {
                        #(#name_map),*
                    }
                }

                pub fn spell(&self) -> Option<crate::types::SpellCard<#game>> {
                    match self {
                        #(#match_spell_map,)*
                        _ => None
                    }
                }

                pub fn is_end(&self) -> bool {
                    matches!(self, #last_variant_pattern)
                }
            }

            #[automatically_derived]
            impl std::fmt::Display for #type_name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    match self {
                        #(#display_map),*
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct GameLocations {
    type_ident: Ident,
    game_type: Ident,
    stage_type: Ident,
    stages: Vec<StageLocations>,
    exclude_stages: Vec<Ident>,
}

impl GameLocations {
    pub fn from_ast(def: &ast::LocationsDef) -> Result<Self, syn::Error> {
        let type_ident = def.type_id.clone();
        let stage_type = def.stage_type.clone();
        let exclude_stages = def.exclude_stages.clone();

        def.stages
            .iter()
            .map(|stage_def| StageLocations::from_ast(def.game_type.clone(), stage_def))
            .collect::<Result<Vec<_>, _>>()
            .map(|stages| Self {
                type_ident,
                game_type: def.game_type.clone(),
                stage_type,
                stages,
                exclude_stages,
            })
    }

    fn has_nonspells(&self) -> bool {
        self.stages.iter().any(|stage| stage.has_nonspells)
    }

    pub fn define_main_enum(&self) -> TokenStream {
        let type_name = &self.type_ident;
        let game = &self.game_type;
        let stage_type = &self.stage_type;

        let resolve_bounds = if self.has_nonspells() {
            quote! {
                T: crate::memory::traits::RunData<#game>,
                <T::StageState as crate::memory::traits::StageData<#game>>::BossState: crate::memory::traits::BossLifebars
            }
        } else {
            quote! {
                T: crate::memory::traits::RunData<#game>
            }
        };

        let variants = self.stages.iter().map(|stage| {
            let stage_type_ident = &stage.type_ident;
            let stage_id = &stage.stage_ident;

            quote! {
                #stage_id(#stage_type_ident)
            }
        });

        let state_ident = format_ident!("stage_state");
        let resolve_match_arms = self.stages.iter().map(|stage| {
            let stage_type_ident = &stage.type_ident;
            let stage_id = &stage.stage_ident;

            quote! {
                #stage_type::#stage_id => #stage_type_ident::resolve(#state_ident).map(Self::#stage_id)
            }
        }).chain(self.exclude_stages.iter().map(|stage_id| {
            quote! { #stage_type::#stage_id => None }
        }));

        let display_map = self.stages.iter().map(|stage| {
            let stage_id = &stage.stage_ident;

            quote! {
                Self::#stage_id(section) => write!(f, "{} {}", &#stage_type::#stage_id, section)
            }
        });

        let spell_match_arms = self.stages.iter().map(|stage| {
            let stage_id = &stage.stage_ident;

            quote! {
                Self::#stage_id(section) => section.spell()
            }
        });

        let is_end_match_arms = self.stages.iter().map(|stage| {
            let stage_id = &stage.stage_ident;

            quote! {
                Self::#stage_id(section) => section.is_end()
            }
        });

        quote! {
            #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
            #[serde(tag = "stage", content = "section", rename_all = "snake_case")]
            pub enum #type_name {
                #(#variants),*
            }

            #[automatically_derived]
            impl #type_name {
                pub fn resolve<T>(state: &T) -> Option<Self>
                    where #resolve_bounds
                {
                    use crate::memory::traits::*;
                    let #state_ident = state.stage();
                    match #state_ident.stage_id() {
                        #(#resolve_match_arms),*
                    }
                }

                pub fn spell(&self) -> Option<crate::types::SpellCard<#game>> {
                    match self {
                        #(#spell_match_arms),*
                    }
                }

                pub fn is_end(&self) -> bool {
                    match self {
                        #(#is_end_match_arms),*
                    }
                }
            }

            #[automatically_derived]
            impl std::fmt::Display for #type_name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    match self {
                        #(#display_map),*
                    }
                }
            }
        }
    }

    pub fn define_sub_enums(&self) -> TokenStream {
        self.stages
            .iter()
            .map(|stage| stage.define_enum())
            .collect()
    }

    pub fn to_definitions(&self) -> TokenStream {
        let mut ret = self.define_sub_enums();
        ret.extend(self.define_main_enum());
        ret
    }
}
