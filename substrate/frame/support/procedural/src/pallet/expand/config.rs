// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::pallet::{parse::GenericKind, Def};
use proc_macro2::{TokenStream, Span};
use quote::quote;
use syn::{parse_quote, Item};

///
/// * Generate default rust doc
/// * Add `Into<Result<Origin>>` and `From<Origin>` to `frame_system::Config::RuntimeOrigin`
///   if needed for authorized call.
pub fn expand_config(def: &mut Def) -> TokenStream {
	add_authorize_constraint(def);

	let config = &def.config;
	let config_item = {
		let item = &mut def.item.content.as_mut().expect("Checked by def parser").1[config.index];
		if let Item::Trait(item) = item {
			item
		} else {
			unreachable!("Checked by config parser")
		}
	};

	config_item.attrs.insert(
		0,
		parse_quote!(
			#[doc = r"
Configuration trait of this pallet.

The main purpose of this trait is to act as an interface between this pallet and the runtime in
which it is embedded in. A type, function, or constant in this trait is essentially left to be
configured by the runtime that includes this pallet.

Consequently, a runtime that wants to include this pallet must implement this trait."
			]
		),
	);

	// we only emit `DefaultConfig` if there are trait items, so an empty `DefaultConfig` is
	// impossible consequently.
	match &config.default_sub_trait {
		Some(default_sub_trait) if default_sub_trait.items.len() > 0 => {
			let trait_items = &default_sub_trait
				.items
				.iter()
				.map(|item| {
					if item.1 {
						if let syn::TraitItem::Type(item) = item.0.clone() {
							let mut item = item.clone();
							item.bounds.clear();
							syn::TraitItem::Type(item)
						} else {
							item.0.clone()
						}
					} else {
						item.0.clone()
					}
				})
				.collect::<Vec<_>>();

			let type_param_bounds = if default_sub_trait.has_system {
				let system = &def.frame_system;
				quote::quote!(: #system::DefaultConfig)
			} else {
				quote::quote!()
			};

			quote!(
				/// Based on [`Config`]. Auto-generated by
				/// [`#[pallet::config(with_default)]`](`frame_support::pallet_macros::config`).
				/// Can be used in tandem with
				/// [`#[register_default_config]`](`frame_support::register_default_config`) and
				/// [`#[derive_impl]`](`frame_support::derive_impl`) to derive test config traits
				/// based on existing pallet config traits in a safe and developer-friendly way.
				///
				/// See [here](`frame_support::pallet_macros::config`) for more information and caveats about
				/// the auto-generated `DefaultConfig` trait and how it is generated.
				pub trait DefaultConfig #type_param_bounds {
					#(#trait_items)*
				}
			)
		},
		_ => Default::default(),
	}
}

/// Add `TryInfo<Origin>` to `frame_system::Config::RuntimeOrigin` if needed for authorized call.
pub fn add_authorize_constraint(def: &mut Def) {
	let Item::Trait(config_item) =
		&mut def.item.content.as_mut().expect("Checked by parser").1[def.config.index]
	else {
		unreachable!("Checked by parser")
	};

	if def
		.call
		.as_ref()
		.map_or(true, |call| call.methods.iter().all(|call| call.authorize.is_none()))
	{
		// No call or no call with authorize.
		return
	}

	let has_instance = def.config.has_instance;
	let frame_system = &def.frame_system;

	let frame_system_supertrait_args = config_item
		.supertraits
		.iter_mut()
		.find_map(|supertrait| {
			if let syn::TypeParamBound::Trait(trait_bound) = supertrait {
				let len = trait_bound.path.segments.len();
				if len >= 2 &&
					trait_bound.path.segments[len - 1].ident == "Config" &&
					trait_bound.path.segments[len - 2].ident == "frame_system"
				{
					Some(&mut trait_bound.path.segments.last_mut().unwrap().arguments)
				} else {
					None
				}
			} else {
				None
			}
		})
		.expect("Checked by parser");

	let origin_gen_kind = if let Some(origin_def) = def.origin.as_ref() {
		GenericKind::from_gens(origin_def.is_generic, has_instance)
			.expect("Consistency is checked by parser")
	} else {
		// Default origin is generic
		GenericKind::from_gens(true, has_instance).expect("Default is generic so no conflict")
	};
	let origin_use_gen = origin_gen_kind.type_use_gen_within_config(Span::call_site());

	let bound_1 = parse_quote!(::core::convert::Into<::core::result::Result<
		Origin<#origin_use_gen>,
		<Self as #frame_system::Config>::RuntimeOrigin
		>>
	);
	let bound_2 = parse_quote!(
		::core::convert::From<Origin<#origin_use_gen>>
	);

	match frame_system_supertrait_args {
		syn::PathArguments::AngleBracketed(args) => {
			let runtime_origin_bounds = args.args.iter_mut().find_map(|bound| {
				if let syn::GenericArgument::Constraint(constraint) = bound {
					if constraint.ident == "RuntimeOrigin" {
						Some(&mut constraint.bounds)
					} else {
						None
					}
				} else {
					None
				}
			});

			if let Some(bounds) = runtime_origin_bounds {
				bounds.push(bound_1);
				bounds.push(bound_2);
			} else {
				args.args.push(parse_quote!(RuntimeOrigin: #bound_1 + #bound_2));
			}
		},
		syn::PathArguments::None =>
			*frame_system_supertrait_args = syn::PathArguments::AngleBracketed(parse_quote!(
				<RuntimeOrigin: #bound_1 + #bound_2>
			)),
		syn::PathArguments::Parenthesized(_) => (), // This is invalid rust we do nothing.
	}
}
