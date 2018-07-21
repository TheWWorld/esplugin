/*
 * This file is part of esplugin
 *
 * Copyright (C) 2017 Oliver Hamlet
 *
 * esplugin is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * esplugin is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with esplugin. If not, see <http://www.gnu.org/licenses/>.
 */
use std::io;

use byteorder::{ByteOrder, LittleEndian};
use nom::le_u32;
use nom::IResult;

use error::Error;
use game_id::GameId;
use subrecord::{parse_subrecord_data_as_u32, Subrecord};

const RECORD_TYPE_LENGTH: u8 = 4;

#[derive(Clone, PartialEq, Eq, Debug, Hash, Default)]
pub struct RecordHeader {
    record_type: String,
    flags: u32,
    form_id: u32,
    size_of_subrecords: u32,
}

impl RecordHeader {
    fn are_subrecords_compressed(&self) -> bool {
        (self.flags & 0x0004_0000) != 0
    }

    pub fn flags(&self) -> u32 {
        self.flags
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Hash, Default)]
pub struct Record {
    header: RecordHeader,
    subrecords: Vec<Subrecord>,
}

impl Record {
    pub fn read_and_validate<T: io::Read>(
        reader: &mut T,
        game_id: GameId,
        expected_type: &[u8],
    ) -> Result<Vec<u8>, Error> {
        let mut content: Vec<u8> = vec![0; header_length(game_id)];
        reader.read_exact(&mut content)?;

        if &content[0..4] != expected_type {
            return Err(Error::ParsingError(
                "Record is not of expected type.".to_owned(),
            ));
        }

        let size_of_subrecords = LittleEndian::read_u32(&content[4..]) as usize;
        if size_of_subrecords > 0 {
            let mut subrecords = vec![0; size_of_subrecords];
            reader.read_exact(&mut subrecords)?;

            content.append(&mut subrecords);
        }

        Ok(content)
    }

    pub fn parse(input: &[u8], game_id: GameId, skip_subrecords: bool) -> IResult<&[u8], Record> {
        record(input, game_id, skip_subrecords)
    }

    pub fn parse_form_id(input: &[u8], game_id: GameId) -> IResult<&[u8], u32> {
        do_parse!(
            input,
            take!(RECORD_TYPE_LENGTH)
                >> size_of_subrecords: le_u32
                >> cond!(game_id == GameId::Morrowind, take!(4))
                >> take!(4)
                >> form_id: cond!(game_id != GameId::Morrowind, le_u32)
                >> cond!(game_id != GameId::Morrowind, take!(4))
                >> cond!(
                    game_id != GameId::Morrowind && game_id != GameId::Oblivion,
                    take!(4)
                ) >> take!(size_of_subrecords) >> (form_id.unwrap_or(0))
        )
    }

    pub fn header(&self) -> &RecordHeader {
        &self.header
    }

    pub fn subrecords(&self) -> &[Subrecord] {
        &self.subrecords
    }
}

fn header_length(game_id: GameId) -> usize {
    match game_id {
        GameId::Morrowind => 16,
        GameId::Oblivion => 20,
        _ => 24,
    }
}

named_args!(record_header(game_id: GameId) <RecordHeader>,
    do_parse!(
        record_type: take_str!(RECORD_TYPE_LENGTH) >>
        size_of_subrecords: le_u32 >>
        cond!(game_id == GameId::Morrowind, take!(4)) >>
        flags: le_u32 >>
        form_id: cond!(game_id != GameId::Morrowind, le_u32) >>
        cond!(game_id != GameId::Morrowind, take!(4)) >>
        cond!(game_id != GameId::Morrowind && game_id != GameId::Oblivion, take!(4)) >>

        (RecordHeader {
            record_type: record_type.to_string(),
            flags,
            form_id: form_id.unwrap_or(0),
            size_of_subrecords,
        })
    )
);

fn record(input: &[u8], game_id: GameId, skip_subrecords: bool) -> IResult<&[u8], Record> {
    let (input1, header) = try_parse!(input, apply!(record_header, game_id));
    let (input2, subrecords_data) = try_parse!(input1, take!(header.size_of_subrecords));

    let subrecords: Vec<Subrecord> = if !skip_subrecords {
        try_parse!(
            subrecords_data,
            apply!(
                parse_subrecords,
                game_id,
                header.are_subrecords_compressed()
            )
        ).1
    } else {
        Vec::new()
    };

    Ok((input2, Record { header, subrecords }))
}

fn parse_subrecords(
    input: &[u8],
    game_id: GameId,
    are_compressed: bool,
) -> IResult<&[u8], Vec<Subrecord>> {
    let mut input1: &[u8] = input;
    let mut subrecords: Vec<Subrecord> = Vec::new();
    let mut large_subrecord_size: u32 = 0;

    while !input1.is_empty() {
        let (input2, subrecord) = try_parse!(
            input1,
            apply!(
                Subrecord::new,
                game_id,
                large_subrecord_size,
                are_compressed
            )
        );
        if subrecord.subrecord_type() == "XXXX" {
            large_subrecord_size = parse_subrecord_data_as_u32(input1)?.1;
        } else {
            large_subrecord_size = 0;
            subrecords.push(subrecord);
        }
        input1 = input2;
    }

    Ok((input1, subrecords))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_should_read_a_record_from_the_given_reader() {
        let data =
            &include_bytes!("../testing-plugins/Skyrim/Data/Blank - Master Dependent.esm")[..0x56];
        let mut reader = io::Cursor::new(data);

        let bytes = Record::read_and_validate(&mut reader, GameId::Skyrim, b"TES4").unwrap();

        let record = Record::parse(&bytes, GameId::Skyrim, false).unwrap().1;

        assert_eq!(0x1, record.header.flags);
        assert_eq!(0, record.header.form_id);
        assert_eq!(5, record.subrecords.len());

        assert_eq!("HEDR", record.subrecords[0].subrecord_type());
        assert_eq!("CNAM", record.subrecords[1].subrecord_type());
        assert_eq!("SNAM", record.subrecords[2].subrecord_type());
        assert_eq!("MAST", record.subrecords[3].subrecord_type());
        assert_eq!("DATA", record.subrecords[4].subrecord_type());
    }

    #[test]
    fn read_and_validate_should_fail_if_the_type_is_unexpected() {
        let data =
            &include_bytes!("../testing-plugins/Skyrim/Data/Blank - Master Dependent.esm")[..0x56];
        let mut reader = io::Cursor::new(data);

        let result = Record::read_and_validate(&mut reader, GameId::Skyrim, b"TES3");
        assert!(result.is_err());
    }

    #[test]
    fn parse_should_read_tes4_header_correctly() {
        let data =
            &include_bytes!("../testing-plugins/Skyrim/Data/Blank - Master Dependent.esm")[..0x56];

        let record = Record::parse(data, GameId::Skyrim, false).unwrap().1;

        assert_eq!(0x1, record.header.flags);
        assert_eq!(0, record.header.form_id);
        assert_eq!(5, record.subrecords.len());

        assert_eq!("HEDR", record.subrecords[0].subrecord_type());
        assert_eq!("CNAM", record.subrecords[1].subrecord_type());
        assert_eq!("SNAM", record.subrecords[2].subrecord_type());
        assert_eq!("MAST", record.subrecords[3].subrecord_type());
        assert_eq!("DATA", record.subrecords[4].subrecord_type());
    }

    #[test]
    fn parse_should_read_tes3_header_correctly() {
        let data = &include_bytes!("../testing-plugins/Morrowind/Data Files/Blank.esm")[..0x144];

        let record = Record::parse(data, GameId::Morrowind, false).unwrap().1;

        assert_eq!(0, record.header.flags);
        assert_eq!(0, record.header.form_id);
        assert_eq!(1, record.subrecords.len());

        assert_eq!("HEDR", record.subrecords[0].subrecord_type());
    }

    #[test]
    fn parse_should_read_oblivion_header_correctly() {
        let data = &include_bytes!("../testing-plugins/Oblivion/Data/Blank.esm")[..0x144];

        let record = Record::parse(data, GameId::Oblivion, false).unwrap().1;

        assert_eq!(1, record.header.flags);
        assert_eq!(0, record.header.form_id);
        assert_eq!(3, record.subrecords.len());

        assert_eq!("HEDR", record.subrecords[0].subrecord_type());
        assert_eq!("CNAM", record.subrecords[1].subrecord_type());
        assert_eq!("SNAM", record.subrecords[2].subrecord_type());
    }

    #[test]
    fn parse_should_obey_skip_subrecords_parameter() {
        let data = &include_bytes!("../testing-plugins/Morrowind/Data Files/Blank.esm")[..0x144];

        let record = Record::parse(data, GameId::Morrowind, true).unwrap().1;

        assert_eq!(0, record.header.flags);
        assert_eq!(0, record.header.form_id);
        assert_eq!(0, record.subrecords.len());
    }

    #[test]
    fn parse_should_read_large_subrecords_correctly() {
        let data = &include_bytes!("../testing-plugins/Skyrim/Data/Blank.esm")[..0x1004C];

        let record = Record::parse(data, GameId::Skyrim, false).unwrap().1;

        assert_eq!(0x1, record.header.flags);
        assert_eq!(0, record.header.form_id);
        assert_eq!(4, record.subrecords.len());

        assert_eq!("HEDR", record.subrecords[0].subrecord_type());
        assert_eq!("CNAM", record.subrecords[1].subrecord_type());
        assert_eq!("SNAM", record.subrecords[2].subrecord_type());
        assert_eq!("ONAM", record.subrecords[3].subrecord_type());
    }

    #[test]
    #[cfg(feature = "compressed-fields")]
    fn parse_should_read_compressed_subrecords_correctly() {
        const DATA: &'static [u8] = &[
            0x42, 0x50, 0x54, 0x44, 0x23, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0xEC, 0x0C,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x2B, 0x00, 0x00, 0x00, 0x42, 0x50, 0x54, 0x4E,
            0x1D, 0x00, 0x19, 0x00, 0x00, 0x00, 0x75, 0xc5, 0x21, 0x0d, 0x00, 0x00, 0x08, 0x05,
            0xd1, 0x6c, 0x6c, 0xdc, 0x57, 0x48, 0x3c, 0xfd, 0x5b, 0x5c, 0x02, 0xd4, 0x6b, 0x32,
            0xb5, 0xdc, 0xa3,
        ];

        let record = Record::parse(DATA, GameId::Skyrim, false).unwrap().1;

        assert_eq!(0xCEC, record.header.form_id);
        assert_eq!(0x00040000, record.header.flags);
        assert_eq!(1, record.subrecords.len());

        let decompressed_data = record.subrecords[0].decompress_data().unwrap();
        assert_eq!(
            "DEFLATE_DEFLATE_DEFLATE_DEFLATE".as_bytes(),
            decompressed_data.as_slice()
        );
    }

    #[test]
    fn parse_form_id_should_return_the_form_id() {
        let data =
            &include_bytes!("../testing-plugins/Skyrim/Data/Blank - Master Dependent.esm")[..0x56];

        let form_id = Record::parse_form_id(data, GameId::Skyrim).unwrap().1;

        assert_eq!(0, form_id);

        let data = &include_bytes!("../testing-plugins/Morrowind/Data Files/Blank.esm")[..0x144];

        let form_id = Record::parse_form_id(data, GameId::Morrowind).unwrap().1;

        assert_eq!(0, form_id);

        let data = &include_bytes!("../testing-plugins/Skyrim/Data/Blank.esp")[0x53..0xEF];

        let form_id = Record::parse_form_id(data, GameId::Skyrim).unwrap().1;

        assert_eq!(0xCEC, form_id);
    }
}
