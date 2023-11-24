use clarity::vm::types::{
    BuffData, ListData, ListTypeData, OptionalData, ResponseData, SequenceData, SequenceSubtype,
    StringSubtype, TupleData, TupleTypeSignature, TypeSignature, Value,
};

use proptest::prelude::*;

pub fn prop_signature() -> impl Strategy<Value = TypeSignature> {
    let leaf = prop_oneof![
        Just(TypeSignature::IntType),
        Just(TypeSignature::UIntType),
        Just(TypeSignature::BoolType),
        // Just(TypeSignature::PrincipalType),
        (0u32..256).prop_map(|s| TypeSignature::SequenceType(SequenceSubtype::BufferType(
            s.try_into().unwrap()
        ))),
        (0u32..256).prop_map(|s| TypeSignature::SequenceType(SequenceSubtype::StringType(
            StringSubtype::ASCII(s.try_into().unwrap())
        ))),
        // TODO: string-utf8
        // TODO: CallableType
        // TODO: trait types
    ];
    leaf.prop_recursive(5, 64, 10, |inner| {
        prop_oneof![
            // optional type: 10% NoType + 90% any other type
            prop_oneof![
                1 => Just(TypeSignature::NoType),
                9 => inner.clone(),
            ]
            .prop_map(|t| TypeSignature::new_option(t).unwrap()),
            // response type: 20% (NoType, any) + 20% (any, NoType) + 60% (any, any)
            prop_oneof![
                1 => inner.clone().prop_map(|ok_ty| TypeSignature::new_response(ok_ty, TypeSignature::NoType).unwrap()),
                1 => inner.clone().prop_map(|err_ty| TypeSignature::new_response(TypeSignature::NoType, err_ty).unwrap()),
                3 => (inner.clone(), inner.clone()).prop_map(|(ok_ty, err_ty)| TypeSignature::new_response(ok_ty, err_ty).unwrap()),
            ],
            // tuple type
            prop::collection::btree_map(r#"[a-zA-Z]{1,16}"#.prop_map(|name| name.try_into().unwrap()), inner.clone(), 1..16).prop_map(|btree| TypeSignature::TupleType(btree.try_into().unwrap())),
            // list type
            (16u32..64, inner.clone()).prop_map(|(s, ty)| (ListTypeData::new_list(ty, s).unwrap()).into()),
            // TODO: ListUnionType
        ]
    })
}

pub fn prop_value(ty: TypeSignature) -> impl Strategy<Value = Value> {
    match ty {
        TypeSignature::NoType => unreachable!(),
        TypeSignature::IntType => int().boxed(),
        TypeSignature::UIntType => uint().boxed(),
        TypeSignature::BoolType => bool().boxed(),
        TypeSignature::OptionalType(ty) => optional(*ty).boxed(),
        TypeSignature::ResponseType(ok_err) => response(ok_err.0, ok_err.1).boxed(),
        TypeSignature::SequenceType(SequenceSubtype::BufferType(size)) => {
            buffer(size.into()).boxed()
        }
        TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::ASCII(size))) => {
            string_ascii(size.into()).boxed()
        }
        TypeSignature::SequenceType(SequenceSubtype::ListType(list_type_data)) => {
            list(list_type_data).boxed()
        }
        TypeSignature::TupleType(tuple_ty) => tuple(tuple_ty).boxed(),
        // TODO
        TypeSignature::PrincipalType => todo!(),
        TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::UTF8(_))) => todo!(),
        TypeSignature::CallableType(_) => todo!(),
        TypeSignature::ListUnionType(_) => todo!(),
        TypeSignature::TraitReferenceType(_) => todo!(),
    }
}

fn int() -> impl Strategy<Value = Value> {
    any::<i128>().prop_map(Value::Int)
}

fn uint() -> impl Strategy<Value = Value> {
    any::<u128>().prop_map(Value::UInt)
}

fn bool() -> impl Strategy<Value = Value> {
    any::<bool>().prop_map(Value::Bool)
}

fn string_ascii(size: u32) -> impl Strategy<Value = Value> {
    let size = size as usize;
    prop::collection::vec(61u8..=123, size..=size).prop_map(|bytes| {
        Value::Sequence(SequenceData::String(clarity::vm::types::CharType::ASCII(
            clarity::vm::types::ASCIIData { data: bytes },
        )))
    })
}

fn buffer(size: u32) -> impl Strategy<Value = Value> {
    let size = size as usize;
    prop::collection::vec(any::<u8>(), size..=size)
        .prop_map(|bytes| Value::Sequence(SequenceData::Buffer(BuffData { data: bytes })))
}

fn optional(inner_ty: TypeSignature) -> impl Strategy<Value = Value> {
    match inner_ty {
        TypeSignature::NoType => Just(Value::none()).boxed(),
        _ => prop::option::of(prop_value(inner_ty))
            .prop_map(|v| {
                Value::Optional(OptionalData {
                    data: v.map(Box::new),
                })
            })
            .boxed(),
    }
}

fn response(ok_ty: TypeSignature, err_ty: TypeSignature) -> impl Strategy<Value = Value> {
    match (ok_ty, err_ty) {
        (TypeSignature::NoType, err_ty) => prop_value(err_ty)
            .prop_map(|err| {
                Value::Response(ResponseData {
                    committed: false,
                    data: Box::new(err),
                })
            })
            .boxed(),
        (ok_ty, TypeSignature::NoType) => prop_value(ok_ty)
            .prop_map(|ok| {
                Value::Response(ResponseData {
                    committed: true,
                    data: Box::new(ok),
                })
            })
            .boxed(),
        (ok_ty, err_ty) => prop::result::maybe_err(prop_value(ok_ty), prop_value(err_ty))
            .prop_map(|res| {
                Value::Response(ResponseData {
                    committed: res.is_ok(),
                    data: res.map_or_else(Box::new, Box::new),
                })
            })
            .boxed(),
    }
}

fn list(list_type_data: ListTypeData) -> impl Strategy<Value = Value> {
    prop::collection::vec(
        prop_value(*list_type_data.entry_type.clone()),
        0..list_type_data.max_len as usize,
    )
    .prop_map(move |v| {
        Value::Sequence(SequenceData::List(ListData {
            data: v,
            type_signature: list_type_data.clone(),
        }))
    })
}

fn tuple(tuple_ty: TupleTypeSignature) -> impl Strategy<Value = Value> {
    let fields: Vec<_> = tuple_ty.get_type_map().keys().cloned().collect();
    let strategies: Vec<_> = tuple_ty
        .get_type_map()
        .values()
        .cloned()
        .map(prop_value)
        .collect();
    strategies.prop_map(move |vec_values| {
        TupleData {
            type_signature: tuple_ty.clone(),
            data_map: fields.clone().into_iter().zip(vec_values).collect(),
        }
        .into()
    })
}

pub fn any_value() -> impl Strategy<Value = Value> {
    prop_signature().prop_flat_map(prop_value)
}
