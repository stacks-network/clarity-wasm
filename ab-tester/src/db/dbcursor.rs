/// Taken in-part from https://github.com/diesel-rs/diesel/issues/1087#issuecomment-517720812
use std::{cell::RefCell, collections::VecDeque, convert::TryInto, marker::PhantomData, rc::Rc};

use color_eyre::eyre::anyhow;
use color_eyre::Result;
use diesel::dsl::{Limit, Offset};
use diesel::prelude::*;
use diesel::query_dsl::methods::{LimitDsl, OffsetDsl};
use diesel::query_dsl::LoadQuery;

/// Get an object that implements the iterator interface.
pub fn stream_results<Record, Model, Query, Conn>(
    query: Query,
    conn: Rc<RefCell<Conn>>,
    buffer_size_hint: usize,
) -> impl Iterator<Item = Result<Model>>
where
    Record: TryInto<Model>,
    Model: Clone,
    Query: OffsetDsl + Clone,
    Offset<Query>: LimitDsl,
    Limit<Offset<Query>>: for<'a> LoadQuery<'a, Conn, Record>,
{
    RecordCursor {
        conn,
        query,
        cursor: 0,
        buffer: VecDeque::with_capacity(buffer_size_hint),
        record_type: PhantomData,
        model_type: PhantomData::default(),
    }
}

/// Represents a pre-fetching cursor for a Diesel query.
pub struct RecordCursor<Record, Model, Query, Conn> {
    conn: Rc<RefCell<Conn>>,
    query: Query,
    /// The index of the next record to fetch from the server
    cursor: usize,
    buffer: VecDeque<Record>,
    record_type: PhantomData<Record>,
    model_type: PhantomData<Model>,
}

/// Implementation of [RecordCursor].
impl<Record, Model, Query, Conn> RecordCursor<Record, Model, Query, Conn>
where
    Record: TryInto<Model>,
    Query: OffsetDsl + Clone,
    Offset<Query>: LimitDsl,
    Limit<Offset<Query>>: for<'a> LoadQuery<'a, Conn, Record>,
    Model: Clone,
{
    pub fn new(query: Query, conn: Rc<RefCell<Conn>>, buffer_size_hint: usize) -> Self {
        Self {
            query,
            conn,
            cursor: 0,
            buffer: VecDeque::with_capacity(buffer_size_hint),
            record_type: PhantomData,
            model_type: PhantomData,
        }
    }

    fn next(&mut self) -> Option<Result<Record>> {
        // if the buffer isn't empty just return an element
        if let Some(v) = self.buffer.pop_front() {
            return Some(Ok(v));
        }

        // fill the buffer
        let fetch_amt = self.buffer.capacity();
        let query = self
            .query
            .clone()
            .offset(self.cursor.try_into().unwrap())
            .limit(fetch_amt.try_into().unwrap());
        self.cursor += fetch_amt;
        let results: Vec<Record> = match query.load(&mut *self.conn.borrow_mut()) {
            Ok(recs) => recs,
            Err(e) => return Some(Err(e.into())),
        };
        for result in results {
            self.buffer.push_back(result);
        }
        // return the first record, or None if there are no more records fetched.
        self.buffer.pop_front().map(Ok)
    }
}

/// Iterator implementation for [RecordCursor].
impl<Record, Model, Query, Conn> Iterator for RecordCursor<Record, Model, Query, Conn>
where
    Record: TryInto<Model>,
    Query: OffsetDsl + Clone,
    Offset<Query>: LimitDsl,
    Limit<Offset<Query>>: for<'a> LoadQuery<'a, Conn, Record>,
    Model: Clone,
{
    type Item = Result<Model>;

    fn next(&mut self) -> Option<Self::Item> {
        // if the buffer isn't empty just return an element
        if let Some(v) = self.buffer.pop_front() {
            let model: Result<Model> = v
                .try_into()
                .map_err(|_| anyhow!("failed to convert record to model"));
            return Some(model);
        }

        // fill the buffer
        let fetch_amt = self.buffer.capacity();
        let query = self
            .query
            .clone()
            .offset(self.cursor.try_into().unwrap())
            .limit(fetch_amt.try_into().unwrap());
        self.cursor += fetch_amt;
        let results: Vec<Record> = match query.load(&mut *self.conn.borrow_mut()) {
            Ok(recs) => recs,
            Err(e) => return Some(Err(e.into())),
        };
        for result in results {
            self.buffer.push_back(result);
        }
        // return the first record, or None if there are no more records fetched.
        self.buffer.pop_front().map(|v| {
            let model: Result<Model> = v
                .try_into()
                .map_err(|_| anyhow!("failed to convert record to model"));
            model
        })
    }
}
