use zmq_generated as gen;

use flatbuffers::{get_root, FlatBufferBuilder, WIPOffset};
use chrono::{Utc, DateTime, prelude::*};

use std::{time::Duration, convert::{TryFrom, TryInto}};

/// High level requests.
pub mod req {
    use super::*;

    /// Used to instanciate a `Historical` request.
    #[derive(Copy, Clone, Debug)]
    pub struct HistoricalArgs<'a> {
        pub routing_stack: &'a [u32],
        pub range: Range,
        pub group: &'a str,
    }

    /// A request for an historical data stream for a group from a specified range.
    #[derive(Copy, Clone, Debug)]
    pub struct Historical<'f> {
        pub(crate) root: gen::Request<'f>,
    }

    impl<'f> Historical<'f> {
        pub fn create(
            builder: &mut FlatBufferBuilder<'f>,
            args: HistoricalArgs,
        ) -> WIPOffset<gen::Request<'f>> {
            let routing_stack =
                builder.create_vector_direct(args.routing_stack);
            let range = args.range.to_root();
            let group = builder.create_string(args.group);

            let mut builder = gen::RequestBuilder::new(builder);
            builder.add_routing_stack(routing_stack);
            builder.add_request_type(gen::RequestType::Historical);
            builder.add_range(&range);
            builder.add_string(group);
            builder.finish()
        }

        pub fn write(
            builder: &mut FlatBufferBuilder<'f>,
            args: HistoricalArgs,
        ) {
            let offset = Self::create(builder, args);
            builder.finish_minimal(offset);
        }

        pub fn routing_stack(&self) -> Option<&[u32]> {
            self.root.routing_stack().map(|r| r.safe_slice())
        }

        pub fn range(&self) -> Option<Range> {
            self.root.range().map(Range::from_root)
        }

        pub fn group(&self) -> Option<&str> {
            self.root.string()
        }
    }

    pub enum GroupAction {
        Join,
        Leave,
    }

    impl GroupAction {
        pub fn from_root(root: gen::GroupAction) -> Option<Self> {
            match root {
                gen::GroupAction::Join => Some(GroupAction::Join),
                gen::GroupAction::Leave => Some(GroupAction::Leave),
                gen::GroupAction::NONE => None,
            }
        }
    }

    /// The variants of requests that modify groups.
    #[derive(Copy, Clone, Debug)]
    pub enum Group<'f> {
        Join(GroupJoin<'f>),
        Leave(GroupLeave<'f>),
    }

    impl<'f> Group<'f> {
        pub fn from_root_unchecked(root: gen::Request<'f>) -> Self {
            let action = GroupAction::from_root(root.group_action()).unwrap();

            match action {
                GroupAction::Join => Group::Join(GroupJoin { root }),
                GroupAction::Leave => Group::Leave(GroupLeave { root }),
            }
        }
    }

    /// Used to instanciate a `GroupJoin` request.
    #[derive(Copy, Clone, Debug)]
    pub struct GroupJoinArgs<'a> {
        pub routing_stack: &'a [u32],
        pub group: &'a libzmq::Group,
        pub bucket_size: Duration,
    }

    /// Add the specified group to the list of handled groups.
    ///
    /// This will cause the database's `Dish` socket to join the specified group,
    /// as well as launch a worker that will handle all the data of said group.
    #[derive(Copy, Clone, Debug)]
    pub struct GroupJoin<'f> {
        pub(crate) root: gen::Request<'f>,
    }

    impl<'f> GroupJoin<'f> {
        pub fn create(
            builder: &mut FlatBufferBuilder<'f>,
            args: GroupJoinArgs,
        ) -> WIPOffset<gen::Request<'f>> {
            let routing_stack =
                builder.create_vector_direct(args.routing_stack);
            let group = builder.create_string(args.group.as_str());

            let mut builder = gen::RequestBuilder::new(builder);
            builder.add_routing_stack(routing_stack);
            builder.add_request_type(gen::RequestType::Group);
            builder.add_string(group);
            builder.add_group_action(gen::GroupAction::Join);
            let bucket_size_ns = u64::try_from(args.bucket_size.as_nanos()).unwrap();
            builder.add_bucket_size_ns(bucket_size_ns);
            builder.finish()
        }

        pub fn write(builder: &mut FlatBufferBuilder<'f>, args: GroupJoinArgs) {
            let offset = Self::create(builder, args);
            builder.finish_minimal(offset);
        }

        pub fn routing_stack(&self) -> Option<&[u32]> {
            self.root.routing_stack().map(|r| r.safe_slice())
        }

        pub fn group(&self) -> Option<&libzmq::Group> {
            self.root.string().map(|s| s.try_into().unwrap())
        }

        pub fn bucket_size(&self) -> Duration {
            Duration::from_nanos(self.root.bucket_size_ns())
        }
    }

    /// Used to instanciate a `GroupLeave` request.
    #[derive(Copy, Clone, Debug)]
    pub struct GroupLeaveArgs<'a> {
        pub routing_stack: &'a [u32],
        pub group: &'a libzmq::Group,
    }

    /// Remove the specified group from the list of handled groups.
    ///
    /// This will cause the database's `Dish` socket to leave the specified group,
    /// as well as terminate the group's assigned worker.
    #[derive(Copy, Clone, Debug)]
    pub struct GroupLeave<'f> {
        pub(crate) root: gen::Request<'f>,
    }

    impl<'f> GroupLeave<'f> {
        pub fn create(
            builder: &mut FlatBufferBuilder<'f>,
            args: GroupLeaveArgs,
        ) -> WIPOffset<gen::Request<'f>> {
            let routing_stack =
                builder.create_vector_direct(args.routing_stack);
            let group = builder.create_string(args.group.as_str());

            let mut builder = gen::RequestBuilder::new(builder);
            builder.add_routing_stack(routing_stack);
            builder.add_request_type(gen::RequestType::Group);
            builder.add_string(group);
            builder.add_group_action(gen::GroupAction::Leave);
            builder.finish()
        }

        pub fn write(
            builder: &mut FlatBufferBuilder<'f>,
            args: GroupLeaveArgs,
        ) {
            let offset = Self::create(builder, args);
            builder.finish_minimal(offset);
        }

        pub fn routing_stack(&self) -> Option<&[u32]> {
            self.root.routing_stack().map(|r| r.safe_slice())
        }

        pub fn group(&self) -> Option<&libzmq::Group> {
            self.root.string().map(|s| s.try_into().unwrap())
        }

    }
} // mod req

/// The content of a `Request`.
#[derive(Clone, Debug)]
pub enum Request<'f> {
    Historical(req::Historical<'f>),
    Group(req::Group<'f>),
}

impl<'f> Request<'f> {
    /// Creates a `Request` from raw slice.
    pub fn from_bytes_unchecked(buf: &'f [u8]) -> Self {
        Self::from_root_unchecked(get_root::<gen::Request>(buf))
    }

    /// Creates a `Request` from its flatbuffers root type.
    pub fn from_root_unchecked(root: gen::Request<'f>) -> Self {
        match root.request_type() {
            gen::RequestType::Historical => {
                let content = req::Historical { root };

                Request::Historical(content)
            }
            gen::RequestType::Group => {
                let content = req::Group::from_root_unchecked(root);

                Request::Group(content)
            }
        }
    }
}

/// High level replies.
pub mod rep {
    use super::*;

    #[derive(Copy, Clone, Debug)]
    pub struct SuccessArgs<'a> {
        /// The id of the request that caused this reply.
        pub routing_stack: &'a [u32],
    }

    /// Informs the client that the associated `Request` was scheduled.
    #[derive(Copy, Clone, Debug)]
    pub struct Success<'f> {
        pub(crate) root: gen::Reply<'f>,
    }

    impl<'f> Success<'f> {
        pub fn create(
            builder: &mut FlatBufferBuilder<'f>,
            args: SuccessArgs,
        ) -> WIPOffset<gen::Reply<'f>> {
            let routing_stack =
                builder.create_vector_direct(args.routing_stack);

            let mut builder = gen::ReplyBuilder::new(builder);
            builder.add_reply_type(gen::ReplyType::Success);
            builder.add_routing_stack(routing_stack);
            builder.finish()
        }

        pub fn write(builder: &mut FlatBufferBuilder<'f>, args: SuccessArgs) {
            let offset = Self::create(builder, args);
            builder.finish_minimal(offset);
        }

        pub fn routing_stack(&self) -> Option<&[u32]> {
            self.root.routing_stack().map(|r| r.safe_slice())
        }
    }

    /// Error codes in case of a `Error`.
    #[derive(Copy, Clone, Debug)]
    pub enum ErrorType {
        MissingField,
        InvalidRange,
        EmptyDataset,
        InvalidGroup,
        DuplicateGroup,
    }

    impl ErrorType {
        fn to_root(self) -> gen::ErrorType {
            match self {
                ErrorType::MissingField => gen::ErrorType::MissingField,
                ErrorType::InvalidRange => gen::ErrorType::InvalidRange,
                ErrorType::EmptyDataset => gen::ErrorType::EmptyDataset,
                ErrorType::InvalidGroup => gen::ErrorType::InvalidGroup,
                ErrorType::DuplicateGroup => gen::ErrorType::DuplicateGroup,
            }
        }

        fn from_root(root: gen::ErrorType) -> Option<Self> {
            match root {
                gen::ErrorType::NONE => None,
                gen::ErrorType::MissingField => Some(ErrorType::MissingField),
                gen::ErrorType::InvalidRange => Some(ErrorType::InvalidRange),
                gen::ErrorType::EmptyDataset => Some(ErrorType::EmptyDataset),
                gen::ErrorType::InvalidGroup => Some(ErrorType::InvalidGroup),
                gen::ErrorType::DuplicateGroup => {
                    Some(ErrorType::DuplicateGroup)
                }
            }
        }
    }

    #[derive(Copy, Clone, Debug)]
    pub struct ErrorArgs<'a> {
        pub routing_stack: &'a [u32],
        pub error_type: ErrorType,
        pub error_msg: &'a str,
    }

    /// Informs the client that the associated `Request` failed.
    ///
    /// It documents the failure via the `error_type` and a corresponding
    /// `error_msg`.
    #[derive(Copy, Clone, Debug)]
    pub struct Error<'f> {
        pub(crate) root: gen::Reply<'f>,
    }

    impl<'f> Error<'f> {
        pub fn create(
            builder: &mut FlatBufferBuilder<'f>,
            args: ErrorArgs,
        ) -> WIPOffset<gen::Reply<'f>> {
            let routing_stack =
                builder.create_vector_direct(args.routing_stack);
            let error_msg = builder.create_string(args.error_msg);

            let mut builder = gen::ReplyBuilder::new(builder);
            builder.add_reply_type(gen::ReplyType::Error);
            builder.add_routing_stack(routing_stack);
            builder.add_error_type(args.error_type.to_root());
            builder.add_error_msg(error_msg);
            builder.finish()
        }

        pub fn write(builder: &mut FlatBufferBuilder<'f>, args: ErrorArgs) {
            let offset = Self::create(builder, args);
            builder.finish_minimal(offset);
        }

        pub fn routing_stack(&self) -> Option<&[u32]> {
            self.root.routing_stack().map(|r| r.safe_slice())
        }

        pub fn error_type(&self) -> Option<ErrorType> {
            ErrorType::from_root(self.root.error_type())
        }

        pub fn error_msg(&self) -> Option<&str> {
            self.root.error_msg()
        }
    }

    #[derive(Copy, Clone, Debug)]
    pub struct HistoricalArgs<'a> {
        /// The id of the request that caused this reply.
        pub routing_stack: &'a [u32],
        pub data_point: DataPointArgs<'a>,
    }

    /// Informs the client that the associated `Request` was scheduled.
    #[derive(Copy, Clone, Debug)]
    pub struct Historical<'f> {
        pub(crate) root: gen::Reply<'f>,
    }

    impl<'f> Historical<'f> {
        pub fn create(
            builder: &mut FlatBufferBuilder<'f>,
            args: HistoricalArgs,
        ) -> WIPOffset<gen::Reply<'f>> {
            let data_point = DataPoint::create(builder, args.data_point);
            let routing_stack =
                builder.create_vector_direct(args.routing_stack);

            let mut builder = gen::ReplyBuilder::new(builder);
            builder.add_reply_type(gen::ReplyType::Historical);
            builder.add_routing_stack(routing_stack);
            builder.add_data_point(data_point);
            builder.finish()
        }

        pub fn write(
            builder: &mut FlatBufferBuilder<'f>,
            args: HistoricalArgs,
        ) {
            let offset = Self::create(builder, args);
            builder.finish_minimal(offset);
        }

        pub fn routing_stack(&self) -> Option<&[u32]> {
            self.root.routing_stack().map(|r| r.safe_slice())
        }

        pub fn data_point(&self) -> Option<DataPoint> {
            self.root.data_point().map(|root| DataPoint { root })
        }
    }

    #[derive(Copy, Clone, Debug)]
    pub struct EndOfStreamArgs<'a> {
        /// The id of the request that caused this reply.
        pub routing_stack: &'a [u32],
    }

    /// Informs the client that the associated `Request` was scheduled.
    #[derive(Copy, Clone, Debug)]
    pub struct EndOfStream<'f> {
        pub(crate) root: gen::Reply<'f>,
    }

    impl<'f> EndOfStream<'f> {
        pub fn create(
            builder: &mut FlatBufferBuilder<'f>,
            args: EndOfStreamArgs,
        ) -> WIPOffset<gen::Reply<'f>> {
            let routing_stack =
                builder.create_vector_direct(args.routing_stack);

            let mut builder = gen::ReplyBuilder::new(builder);
            builder.add_reply_type(gen::ReplyType::EndOfStream);
            builder.add_routing_stack(routing_stack);
            builder.finish()
        }

        pub fn write(
            builder: &mut FlatBufferBuilder<'f>,
            args: EndOfStreamArgs,
        ) {
            let offset = Self::create(builder, args);
            builder.finish_minimal(offset);
        }

        pub fn routing_stack(&self) -> Option<&[u32]> {
            self.root.routing_stack().map(|r| r.safe_slice())
        }
    }
}

#[derive(Clone, Debug)]
pub enum Reply<'f> {
    Success(rep::Success<'f>),
    Error(rep::Error<'f>),
    Historical(rep::Historical<'f>),
    EndOfStream(rep::EndOfStream<'f>),
}

impl<'f> Reply<'f> {
    pub fn from_bytes_unchecked(slice: &'f [u8]) -> Self {
        let root = get_root::<gen::Reply>(slice);
        Self::from_root_unchecked(root)
    }

    pub fn from_root_unchecked(root: gen::Reply<'f>) -> Self {
        match root.reply_type() {
            gen::ReplyType::Error => Reply::Error(rep::Error { root }),
            gen::ReplyType::Success => Reply::Success(rep::Success { root }),
            gen::ReplyType::Historical => {
                Reply::Historical(rep::Historical { root })
            }
            gen::ReplyType::EndOfStream => {
                Reply::EndOfStream(rep::EndOfStream { root })
            }
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct DataPointArgs<'a> {
    pub dt: DateTime<Utc>,
    pub data: &'a [u8],
}

/// A timestamped timeseries binary datapoint.
///
/// The caller must know how to interprete these bytes.
#[derive(Copy, Clone, Debug)]
pub struct DataPoint<'f> {
    root: gen::DataPoint<'f>,
}

impl<'f> DataPoint<'f> {
    pub fn from_bytes_unchecked(slice: &'f [u8]) -> Self {
        let root = get_root::<gen::DataPoint>(slice);
        Self::from_root_unchecked(root)
    }

    pub fn from_root_unchecked(root: gen::DataPoint<'f>) -> Self {
        Self { root }
    }

    pub fn create(
        builder: &mut FlatBufferBuilder<'f>,
        args: DataPointArgs,
    ) -> WIPOffset<gen::DataPoint<'f>> {
        let data = builder.create_vector_direct(args.data);

        let args = gen::DataPointArgs {
            ns_ts: args.dt.timestamp_nanos(),
            data: Some(data),
        };

        gen::DataPoint::create(builder, &args)
    }

    pub fn write(builder: &mut FlatBufferBuilder<'f>, args: DataPointArgs) {
        let offset = Self::create(builder, args);
        builder.finish_minimal(offset);
    }

    pub fn dt(&self) -> DateTime<Utc> {
        Utc.timestamp_nanos(self.root.ns_ts())
    }

    /// The underlying data.
    pub fn data(&self) -> Option<&[u8]> {
        self.root.data()
    }
}

#[derive(Copy, Clone, Debug)]
pub struct DataPointVecArgs<'a, 'f> {
    pub vec: &'a [WIPOffset<gen::DataPoint<'f>>],
}

/// A vector of timestamped timeseries binary datapoint.
///
/// The caller must know how to interprete these bytes.
#[derive(Copy, Clone, Debug)]
pub struct DataPointVec<'f> {
    root: gen::DataPointVec<'f>,
}

impl<'f> DataPointVec<'f> {
    pub const FILE_IDENTIFIER: &'static str = "ldpv";

    pub fn from_bytes_unchecked(slice: &'f [u8]) -> Self {
        let root = get_root::<gen::DataPointVec>(slice);
        Self::from_root_unchecked(root)
    }

    pub fn from_root_unchecked(root: gen::DataPointVec<'f>) -> Self {
        Self { root }
    }

    pub fn create<'a>(
        builder: &mut FlatBufferBuilder<'f>,
        args: DataPointVecArgs<'a, 'f>,
    ) -> WIPOffset<gen::DataPointVec<'f>> {
        let vec = builder.create_vector(args.vec);

        let args = gen::DataPointVecArgs { vec: Some(vec) };

        gen::DataPointVec::create(builder, &args)
    }

    pub fn write<'a>(
        builder: &mut FlatBufferBuilder<'f>,
        args: DataPointVecArgs<'a, 'f>,
    ) {
        let offset = Self::create(builder, args);
        builder.finish_minimal(offset);
    }

    /// The underlying data.
    pub fn vec(&self) -> Option<impl Iterator<Item = DataPoint>> {
        self.root
            .vec()
            .map(|v| v.iter().map(|root| DataPoint { root }))
    }
}
