use flexbuffers::{
    Builder,
    MapBuilder,
    Pushable,
    VectorBuilder,
};

pub trait FlexBuilder {
    fn push<P: Pushable>(&mut self, value: P);
    fn start_vector(&mut self) -> VectorBuilder<'_>;
    fn start_map(&mut self) -> MapBuilder<'_>;
}

impl FlexBuilder for Builder {
    fn push<P: Pushable>(&mut self, value: P) {
        self.build_singleton(value);
    }

    fn start_vector(&mut self) -> VectorBuilder<'_> {
        self.start_vector()
    }

    fn start_map(&mut self) -> MapBuilder<'_> {
        self.start_map()
    }
}

impl FlexBuilder for VectorBuilder<'_> {
    fn push<P: Pushable>(&mut self, value: P) {
        self.push(value)
    }

    fn start_vector(&mut self) -> VectorBuilder<'_> {
        self.start_vector()
    }

    fn start_map(&mut self) -> MapBuilder<'_> {
        self.start_map()
    }
}

impl<'a> FlexBuilder for (&'a str, &'a mut MapBuilder<'_>) {
    fn push<P: Pushable>(&mut self, value: P) {
        self.1.push(self.0, value);
    }

    fn start_vector(&mut self) -> VectorBuilder<'_> {
        self.1.start_vector(self.0)
    }

    fn start_map(&mut self) -> MapBuilder<'_> {
        self.1.start_map(self.0)
    }
}
