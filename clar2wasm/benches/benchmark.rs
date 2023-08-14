use criterion::{criterion_group, criterion_main, Criterion};
use std::borrow::BorrowMut;
use wasmtime::{Engine, Instance, Module, Store, Val};

fn add(c: &mut Criterion) {
    c.bench_function("add", |b| {
        let standard_lib = include_str!("../src/standard/standard.wat");
        let engine = Engine::default();
        let mut store = Store::new(&engine, ());
        let module = Module::new(&engine, standard_lib).unwrap();
        let instance = Instance::new(&mut store.borrow_mut(), &module, &[]).unwrap();
        let add = instance
            .get_func(&mut store.borrow_mut(), "add-int")
            .unwrap();

        b.iter(|| {
            let mut results = [Val::I64(0), Val::I64(0)];
            add.call(
                &mut store.borrow_mut(),
                &[Val::I64(0), Val::I64(42), Val::I64(0), Val::I64(12345)],
                &mut results,
            )
            .unwrap();
        })
    });
}

fn mul(c: &mut Criterion) {
    c.bench_function("mul", |b| {
        let standard_lib = include_str!("../src/standard/standard.wat");
        let engine = Engine::default();
        let mut store = Store::new(&engine, ());
        let module = Module::new(&engine, standard_lib).unwrap();
        let instance = Instance::new(&mut store.borrow_mut(), &module, &[]).unwrap();
        let mul = instance
            .get_func(&mut store.borrow_mut(), "mul-uint")
            .unwrap();

        b.iter(|| {
            let mut results = [Val::I64(0), Val::I64(0)];
            mul.call(
                &mut store.borrow_mut(),
                &[Val::I64(0), Val::I64(-1), Val::I64(0), Val::I64(-1)],
                &mut results,
            )
            .unwrap();
        })
    });
}

criterion_group!(arithmetic, add, mul);
criterion_main!(arithmetic);
