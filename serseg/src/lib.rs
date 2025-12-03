pub mod builder;
pub mod field;
pub mod prelude;
pub(crate) mod tracker;

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use u24::u24;

    use crate::prelude::*;

    type Builder = SerialBuilder<ExampleSectorKey>;
    type SectorBuilder = SerialSectorBuilder<ExampleSectorKey>;

    #[derive(Debug, Clone, Hash, PartialEq, Eq)]
    enum ExampleSectorKey {
        First,
        Second,
        Third,
    }

    #[tokio::test]
    async fn sector_string() {
        let expected = b"This is a test\x00";
        let mut buffer = Cursor::new(Vec::with_capacity(expected.len()));

        Builder::default()
            .sector(
                ExampleSectorKey::First,
                SectorBuilder::default().string("This is a test"),
            )
            .build(&mut buffer)
            .await
            .unwrap();

        assert_eq!(buffer.into_inner(), expected);
    }

    #[tokio::test]
    async fn sector_u24() {
        let expected = [0x12, 0x34, 0x56];
        let mut buffer = Cursor::new(Vec::with_capacity(expected.len()));

        Builder::default()
            .sector(
                ExampleSectorKey::First,
                SectorBuilder::default().u24(u24::from_le_bytes([0x12, 0x34, 0x56])),
            )
            .build(&mut buffer)
            .await
            .unwrap();

        assert_eq!(buffer.into_inner(), expected);
    }

    #[tokio::test]
    async fn sector_dynamic() {
        let expected = b"\xFF\x06\x00\x00\x13\x00\x00first string\x00second string\x00";
        let mut buffer = Cursor::new(Vec::with_capacity(expected.len()));

        Builder::default()
            .sector(ExampleSectorKey::First, SectorBuilder::default().u8(0xFF))
            .sector(
                ExampleSectorKey::Second,
                SectorBuilder::default()
                    .dynamic_u24(ExampleSectorKey::Second, ExampleSectorKey::Third, 0)
                    .dynamic_u24(ExampleSectorKey::Second, ExampleSectorKey::Third, 1),
            )
            .sector(
                ExampleSectorKey::Third,
                SectorBuilder::default()
                    .string("first string")
                    .string("second string"),
            )
            .build(&mut buffer)
            .await
            .unwrap();

        assert_eq!(buffer.into_inner(), expected);
    }

    #[tokio::test]
    async fn sector_fill() {
        let expected = [
            b'T', b'e', b's', b't', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xFF,
        ];
        let mut buffer = Cursor::new(Vec::with_capacity(expected.len()));

        Builder::default()
            .sector_default(ExampleSectorKey::First)
            .sector(
                ExampleSectorKey::Second,
                SectorBuilder::default()
                    .string("Test")
                    .fill(ExampleSectorKey::First, 16)
                    .u8(0xFF),
            )
            .build(&mut buffer)
            .await
            .unwrap();

        assert_eq!(buffer.into_inner(), expected);
    }

    #[tokio::test]
    async fn sector_fill_end() {
        let expected = b"Test\x00";
        let mut buffer = Cursor::new(Vec::with_capacity(expected.len()));

        Builder::default()
            .sector_default(ExampleSectorKey::First)
            .sector(
                ExampleSectorKey::Second,
                SectorBuilder::default()
                    .string("Test")
                    .fill(ExampleSectorKey::First, 16),
            )
            .build(&mut buffer)
            .await
            .unwrap();

        assert_eq!(buffer.into_inner(), expected);
    }

    #[tokio::test]
    async fn sector_fill_overflow() {
        let mut buffer = Cursor::new(Vec::new());

        let result = Builder::default()
            .sector_default(ExampleSectorKey::First)
            .sector(
                ExampleSectorKey::Second,
                SectorBuilder::default()
                    .string("Test")
                    .fill(ExampleSectorKey::First, 2),
            )
            .build(&mut buffer)
            .await;

        assert!(result.is_err());
    }
}
