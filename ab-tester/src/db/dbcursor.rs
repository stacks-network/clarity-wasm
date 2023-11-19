/// Taken in-part from https://github.com/diesel-rs/diesel/issues/1087#issuecomment-517720812
use std::{collections::VecDeque, convert::TryInto, marker::PhantomData};

use diesel::{
    dsl::{Limit, Offset},
    prelude::*,
    query_dsl::{
        methods::{LimitDsl, OffsetDsl},
        LoadQuery,
    }, connection::LoadConnection
};
use color_eyre::Result;

/// Get an object that implements the iterator interface.
pub fn stream_results<'conn, Record, Model, Query, Conn>(
    query: Query,
    conn: &'conn mut Conn,
    buffer_size_hint: usize,
) -> impl Iterator<Item = Result<Record>> + 'conn
where
    Record: 'conn + TryInto<Model>,
    Model: 'conn + Clone,
    Query: OffsetDsl + Clone + 'conn,
    Offset<Query>: LimitDsl,
    Limit<Offset<Query>>: LoadQuery<'conn, Conn, Record>,
{
    RecordCursor {
        conn,
        query,
        cursor: 0,
        buffer: VecDeque::with_capacity(buffer_size_hint),
        record_type: PhantomData,
        model_type: PhantomData::default()
    }
}

pub struct RecordIter<T>(Box<dyn Iterator<Item = Result<T>>>);

impl<Record> RecordIter<Record> {
    pub fn new<'conn, Model, Query, Conn>(
        inner: RecordCursor<'conn, Record, Model, Query, Conn>
    ) -> Self
    where
        Record: 'conn + TryInto<Model>,
        Model: 'conn + Clone,
        Query: OffsetDsl + Clone,
        Offset<Query>: LimitDsl,
        Limit<Offset<Query>>: LoadQuery<'conn, Conn, Record>,
    {
        
        let iter = inner.into_iter();
        Self(Box::new(iter))
    }
}

impl<Record> Iterator for RecordIter<Record> {
    type Item = Result<Record>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
    
}

pub struct RecordCursor<'conn, Record, Model, Query, Conn> {
    conn: &'conn mut Conn,
    query: Query,
    /// The index of the next record to fetch from the server
    cursor: usize,
    buffer: VecDeque<Record>,
    record_type: PhantomData<Record>,
    model_type: PhantomData<Model>,
}

impl<'conn, Record, Model, Query, Conn> RecordCursor<'conn, Record, Model, Query, Conn>
where
    Record: 'conn + TryInto<Model>,
    Query: OffsetDsl + Clone + 'conn,
    Offset<Query>: LimitDsl,
    Limit<Offset<Query>>: LoadQuery<'conn, Conn, Record>,
    Model: 'conn + Clone
{
    pub fn new(
        query: Query,
        conn: &'conn mut Conn,
        buffer_size_hint: usize
    ) -> Self {
        Self {
            query,
            conn,
            cursor: 0,
            buffer: VecDeque::with_capacity(buffer_size_hint),
            record_type: PhantomData,
            model_type: PhantomData
        }
    }

    pub fn next(&mut self) -> Option<Result<Record>> {
        // if the buffer isn't empty just return an element
        if let Some(v) = self.buffer.pop_front() { 
            return Some(Ok(v)) 
        }

        // fill the buffer
        let fetch_amt = self.buffer.capacity();
        let query = self
            .query
            .clone()
            .offset(self.cursor.try_into().unwrap())
            .limit(fetch_amt.try_into().unwrap());
        self.cursor += fetch_amt;
        let results: Vec<Record> = match query.load(self.conn) {
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

impl<'conn, Record, Model, Query, Conn> Iterator for RecordCursor<'conn, Record, Model, Query, Conn>
where
    Record: 'conn + TryInto<Model>,
    Query: OffsetDsl + Clone,
    Offset<Query>: LimitDsl,
    Limit<Offset<Query>>: LoadQuery<'conn, Conn, Record>,
    Model: 'conn + Clone
{
    type Item = Result<Record>;

    fn next(&mut self) -> Option<Self::Item> {
        // if the buffer isn't empty just return an element
        if let Some(v) = self.buffer.pop_front() { 
            return Some(Ok(v)) 
        }

        // fill the buffer
        let fetch_amt = self.buffer.capacity();
        let query = self
            .query
            .clone()
            .offset(self.cursor.try_into().unwrap())
            .limit(fetch_amt.try_into().unwrap());
        self.cursor += fetch_amt;
        let results: Vec<Record> = match query.load(self.conn) {
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

fn test() {
    let query = crate::db::schema::sortition::snapshots::table;
    let mut conn = SqliteConnection::establish("").expect("hi");

    let result = 
        stream_results::<crate::db::model::sortition_db::Snapshot, crate::types::Snapshot, _, _>(
            query, 
            &mut conn, 
            100
        );

    for item in result {
        eprintln!("{:?}", item);
    }
}