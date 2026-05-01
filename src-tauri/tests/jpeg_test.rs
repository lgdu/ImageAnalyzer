use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[allow(dead_code)]
fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

// ============================================================================
// Test JPEG builders
// ============================================================================

/// Build a minimal valid JPEG with the given structure.
fn build_minimal_jpeg(width: u32, height: u32) -> Vec<u8> {
    let mut data = Vec::new();

    // SOI
    data.extend_from_slice(&[0xFF, 0xD8]);

    // APP0 (JFIF)
    let jfif: Vec<u8> = vec![
        b'J', b'F', b'I', b'F', 0x00, // Identifier
        0x01, 0x02, // Version 1.2
        0x00, // Units: no units
        0x00, 0x01, // X density = 1
        0x00, 0x01, // Y density = 1
        0x00, 0x00, // Thumbnail dimensions
    ];
    write_jpeg_segment(&mut data, 0xE0, &jfif);

    // DQT (single quantization table)
    let mut dqt = vec![0x00]; // Table ID 0, 8-bit
    dqt.extend(vec![10u8; 64]); // Simple quantization table
    write_jpeg_segment(&mut data, 0xDB, &dqt);

    // SOF0 (Baseline DCT)
    let sof0: Vec<u8> = vec![
        0x08, // Precision (8 bits)
        ((height >> 8) & 0xFF) as u8,
        (height & 0xFF) as u8, // Height
        ((width >> 8) & 0xFF) as u8,
        (width & 0xFF) as u8, // Width
        0x03,                 // Number of components
        0x01,
        0x22,
        0x00, // Component 1: Y, sampling 2:2, QT 0
        0x02,
        0x11,
        0x00, // Component 2: Cb, sampling 1:1, QT 0
        0x03,
        0x11,
        0x00, // Component 3: Cr, sampling 1:1, QT 0
    ];
    write_jpeg_segment(&mut data, 0xC0, &sof0);

    // DHT (minimal Huffman table)
    let dht: Vec<u8> = vec![
        0x00, // Table class 0, ID 0 (DC table 0)
        0, 1, 5, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0,
        0, // 16 bytes: number of codes per length
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B,
    ];
    write_jpeg_segment(&mut data, 0xC4, &dht);

    // SOS (Start of Scan) - minimal header
    let sos: Vec<u8> = vec![
        0x03, // Number of components
        0x01, 0x00, // Component 1, DC/AC tables 0
        0x02, 0x11, // Component 2, DC/AC tables 1
        0x03, 0x11, // Component 3, DC/AC tables 1
        0x00, 0x3F, 0x00, // Spectral selection, approx
    ];
    write_jpeg_segment(&mut data, 0xDA, &sos);

    // Minimal entropy data (just a few bytes, not valid JPEG data but enough for parser)
    data.push(0x00);
    data.push(0xFF);
    data.push(0x00); // Stuffed byte

    // EOI
    data.extend_from_slice(&[0xFF, 0xD9]);

    data
}

/// Build a JPEG with EXIF data embedded in APP1
/// Uses big-endian TIFF (MM) with numeric-only inline values for reliability
fn build_jpeg_with_exif(width: u32, height: u32) -> Vec<u8> {
    let mut data = Vec::new();

    // SOI
    data.extend_from_slice(&[0xFF, 0xD8]);

    // APP0 (JFIF)
    let jfif: Vec<u8> = vec![
        b'J', b'F', b'I', b'F', 0x00, 0x01, 0x02, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00,
    ];
    write_jpeg_segment(&mut data, 0xE0, &jfif);

    // APP1 (EXIF) - big-endian TIFF ("MM") with only inline numeric values
    let mut exif_data = Vec::new();
    exif_data.extend_from_slice(b"Exif\0\0");

    // TIFF header (big-endian)
    let ifd0_offset: u32 = 8;
    let num_ifd_entries: u16 = 4;

    // Build IFD0 data (big-endian)
    let mut ifd0 = Vec::new();
    ifd0.extend_from_slice(&num_ifd_entries.to_be_bytes());

    // ImageWidth: LONG (inline value)
    ifd0.extend_from_slice(&0x0100u16.to_be_bytes());
    ifd0.extend_from_slice(&4u16.to_be_bytes()); // LONG
    ifd0.extend_from_slice(&1u32.to_be_bytes()); // count = 1
    ifd0.extend_from_slice(&width.to_be_bytes());

    // ImageHeight: LONG (inline value)
    ifd0.extend_from_slice(&0x0101u16.to_be_bytes());
    ifd0.extend_from_slice(&4u16.to_be_bytes());
    ifd0.extend_from_slice(&1u32.to_be_bytes());
    ifd0.extend_from_slice(&height.to_be_bytes());

    // Orientation: SHORT (inline value)
    ifd0.extend_from_slice(&0x0112u16.to_be_bytes());
    ifd0.extend_from_slice(&3u16.to_be_bytes()); // SHORT
    ifd0.extend_from_slice(&1u32.to_be_bytes()); // count = 1
    ifd0.extend_from_slice(&1u32.to_be_bytes()); // value = 1

    // XResolution: RATIONAL (inline value = 72/1)
    ifd0.extend_from_slice(&0x011Au16.to_be_bytes());
    ifd0.extend_from_slice(&5u16.to_be_bytes()); // RATIONAL
    ifd0.extend_from_slice(&1u32.to_be_bytes()); // count = 1
                                                 // Offset to rational value (stored inline as offset to self area)
                                                 // Actually, RATIONAL is 8 bytes, can't fit inline. Use offset.
                                                 // We'll store offset to a small area after IFD
    let rational_offset: u32 = 8 + 2 + (num_ifd_entries as u32) * 12 + 4;
    ifd0.extend_from_slice(&rational_offset.to_be_bytes());

    // Next IFD = null
    ifd0.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);

    // Rational data: 72/1 as two big-endian u32 values
    let rational_data: Vec<u8> = vec![
        0x00, 0x00, 0x00, 0x48, // numerator = 72
        0x00, 0x00, 0x00, 0x01, // denominator = 1
    ];

    // Assemble
    exif_data.extend_from_slice(&[0x4D, 0x4D]); // "MM" big-endian
    exif_data.extend_from_slice(&[0x00, 0x2A]); // TIFF magic
    exif_data.extend_from_slice(&ifd0_offset.to_be_bytes());
    exif_data.extend_from_slice(&ifd0);
    exif_data.extend_from_slice(&rational_data);

    write_jpeg_segment(&mut data, 0xE1, &exif_data);

    // DQT
    let mut dqt = vec![0x00];
    dqt.extend(vec![10u8; 64]);
    write_jpeg_segment(&mut data, 0xDB, &dqt);

    // SOF0
    let sof0: Vec<u8> = vec![
        0x08,
        ((height >> 8) & 0xFF) as u8,
        (height & 0xFF) as u8,
        ((width >> 8) & 0xFF) as u8,
        (width & 0xFF) as u8,
        0x03,
        0x01,
        0x22,
        0x00,
        0x02,
        0x11,
        0x00,
        0x03,
        0x11,
        0x00,
    ];
    write_jpeg_segment(&mut data, 0xC0, &sof0);

    // SOS
    let sos: Vec<u8> = vec![0x03, 0x01, 0x00, 0x02, 0x11, 0x03, 0x11, 0x00, 0x3F, 0x00];
    write_jpeg_segment(&mut data, 0xDA, &sos);

    data.push(0x00);
    data.push(0xFF);
    data.push(0x00);

    // EOI
    data.extend_from_slice(&[0xFF, 0xD9]);

    data
}

/// Build a JPEG with XMP data in APP1
fn build_jpeg_with_xmp(width: u32, height: u32) -> Vec<u8> {
    let mut data = Vec::new();

    // SOI
    data.extend_from_slice(&[0xFF, 0xD8]);

    // APP0 (JFIF)
    let jfif: Vec<u8> = vec![
        b'J', b'F', b'I', b'F', 0x00, 0x01, 0x02, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00,
    ];
    write_jpeg_segment(&mut data, 0xE0, &jfif);

    // APP1 (XMP)
    let mut xmp_data = Vec::new();
    xmp_data.extend_from_slice(b"http://ns.adobe.com/xap/1.0/\0");
    let xmp_xml = r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:Description xmlns:xmp="http://ns.adobe.com/xap/1.0/" xmlns:dc="http://purl.org/dc/elements/1.1/" rdf:about="">
<xmp:CreatorTool>Test Camera</xmp:CreatorTool>
<xmp:Rating>4</xmp:Rating>
<dc:description><rdf:Alt><rdf:li xml:lang="x-default">A test JPEG with XMP</rdf:li></rdf:Alt></dc:description>
</rdf:Description>
</rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#;
    xmp_data.extend_from_slice(xmp_xml.as_bytes());
    write_jpeg_segment(&mut data, 0xE1, &xmp_data);

    // DQT
    let mut dqt = vec![0x00];
    dqt.extend(vec![10u8; 64]);
    write_jpeg_segment(&mut data, 0xDB, &dqt);

    // SOF0
    let sof0: Vec<u8> = vec![
        0x08,
        ((height >> 8) & 0xFF) as u8,
        (height & 0xFF) as u8,
        ((width >> 8) & 0xFF) as u8,
        (width & 0xFF) as u8,
        0x03,
        0x01,
        0x22,
        0x00,
        0x02,
        0x11,
        0x00,
        0x03,
        0x11,
        0x00,
    ];
    write_jpeg_segment(&mut data, 0xC0, &sof0);

    // SOS
    let sos: Vec<u8> = vec![0x03, 0x01, 0x00, 0x02, 0x11, 0x03, 0x11, 0x00, 0x3F, 0x00];
    write_jpeg_segment(&mut data, 0xDA, &sos);

    data.push(0x00);
    data.push(0xFF);
    data.push(0x00);

    // EOI
    data.extend_from_slice(&[0xFF, 0xD9]);

    data
}

/// Build a JPEG with IPTC data in APP13
fn build_jpeg_with_iptc(width: u32, height: u32) -> Vec<u8> {
    let mut data = Vec::new();

    // SOI
    data.extend_from_slice(&[0xFF, 0xD8]);

    // APP0 (JFIF)
    let jfif: Vec<u8> = vec![
        b'J', b'F', b'I', b'F', 0x00, 0x01, 0x02, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00,
    ];
    write_jpeg_segment(&mut data, 0xE0, &jfif);

    // APP13 (Photoshop 3.0 + IPTC)
    let mut app13_data = Vec::new();
    app13_data.extend_from_slice(b"Photoshop 3.0\0");

    // Build IPTC data
    let mut iptc = Vec::new();

    // Dataset: ObjectName (2:5)
    iptc.extend_from_slice(&[0x1C, 0x02, 0x05]); // Record 2, Dataset 5 (ObjectName)
    let object_name = b"Test Image";
    iptc.push(object_name.len() as u8);
    iptc.extend_from_slice(object_name);

    // Dataset: Keywords (2:30)
    iptc.extend_from_slice(&[0x1C, 0x02, 0x1E]); // Record 2, Dataset 30 (Keywords)
    let keywords = b"test";
    iptc.push(keywords.len() as u8);
    iptc.extend_from_slice(keywords);

    // Dataset: Caption/Abstract (2:120)
    iptc.extend_from_slice(&[0x1C, 0x02, 0x78]); // Record 2, Dataset 120 (Caption)
    let caption = b"This is a test caption for JPEG IPTC parsing";
    iptc.push(caption.len() as u8);
    iptc.extend_from_slice(caption);

    // Dataset: By-line (2:80)
    iptc.extend_from_slice(&[0x1C, 0x02, 0x50]); // Record 2, Dataset 80 (By-line)
    let byline = b"Test Author";
    iptc.push(byline.len() as u8);
    iptc.extend_from_slice(byline);

    // Dataset: City (2:90)
    iptc.extend_from_slice(&[0x1C, 0x02, 0x5A]); // Record 2, Dataset 90 (City)
    let city = b"Test City";
    iptc.push(city.len() as u8);
    iptc.extend_from_slice(city);

    // Wrap IPTC in Photoshop 8BIM block
    // Signature "8BIM" (4) + Resource ID 1028 (2) + Name (pascal: len=0) (1) + padding (1) + Data size (4) + Data
    let mut ps_block = Vec::new();
    ps_block.extend_from_slice(b"8BIM");
    ps_block.extend_from_slice(&[0x04, 0x04]); // Resource ID 1028 (big-endian)
    ps_block.push(0x00); // Name length = 0
    ps_block.push(0x00); // Pad to even
    ps_block.extend_from_slice(&(iptc.len() as u32).to_be_bytes());
    ps_block.extend_from_slice(&iptc);
    if iptc.len() % 2 == 1 {
        ps_block.push(0x00); // Pad data to even
    }

    app13_data.extend_from_slice(&ps_block);
    write_jpeg_segment(&mut data, 0xED, &app13_data);

    // DQT
    let mut dqt = vec![0x00];
    dqt.extend(vec![10u8; 64]);
    write_jpeg_segment(&mut data, 0xDB, &dqt);

    // SOF0
    let sof0: Vec<u8> = vec![
        0x08,
        ((height >> 8) & 0xFF) as u8,
        (height & 0xFF) as u8,
        ((width >> 8) & 0xFF) as u8,
        (width & 0xFF) as u8,
        0x03,
        0x01,
        0x22,
        0x00,
        0x02,
        0x11,
        0x00,
        0x03,
        0x11,
        0x00,
    ];
    write_jpeg_segment(&mut data, 0xC0, &sof0);

    // SOS
    let sos: Vec<u8> = vec![0x03, 0x01, 0x00, 0x02, 0x11, 0x03, 0x11, 0x00, 0x3F, 0x00];
    write_jpeg_segment(&mut data, 0xDA, &sos);

    data.push(0x00);
    data.push(0xFF);
    data.push(0x00);

    // EOI
    data.extend_from_slice(&[0xFF, 0xD9]);

    data
}

/// Write a JPEG segment: 0xFF + marker_byte + length (big-endian u16) + data
fn write_jpeg_segment(data: &mut Vec<u8>, marker: u8, segment_data: &[u8]) {
    data.push(0xFF);
    data.push(marker);
    let length = (segment_data.len() + 2) as u16; // length includes the 2 length bytes
    data.extend_from_slice(&length.to_be_bytes());
    data.extend_from_slice(segment_data);
}

// ============================================================================
// Tests
// ============================================================================

#[test]
fn parses_valid_jpeg_file() {
    let jpeg_data = build_minimal_jpeg(100, 80);
    let tmp = std::env::temp_dir().join("test_minimal.jpg");
    fs::write(&tmp, &jpeg_data).unwrap();

    let result = image_analyzer::analyzer::jpeg_parser::analyze_jpeg(tmp.to_str().unwrap());
    let analysis = result.expect("Should parse valid JPEG");

    assert_eq!(analysis.file_name, "test_minimal.jpg");
    assert_eq!(analysis.width, 100, "Width should be 100");
    assert_eq!(analysis.height, 80, "Height should be 80");
    assert!(
        !analysis.structure.is_empty(),
        "Should have marker structure"
    );
    assert!(analysis.analysis_errors.is_empty(), "Should have no errors");

    fs::remove_file(&tmp).ok();
}

#[test]
fn jpeg_has_soi_marker() {
    let jpeg_data = build_minimal_jpeg(64, 64);
    let tmp = std::env::temp_dir().join("test_soi.jpg");
    fs::write(&tmp, &jpeg_data).unwrap();

    let analysis = image_analyzer::analyzer::jpeg_parser::analyze_jpeg(tmp.to_str().unwrap())
        .expect("Should parse");

    let soi = analysis
        .structure
        .iter()
        .find(|b| b.name == "SOI")
        .expect("JPEG should have SOI marker");

    assert_eq!(soi.offset, 0, "SOI should be at offset 0");
    assert_eq!(soi.length, 2, "SOI should be 2 bytes");

    fs::remove_file(&tmp).ok();
}

#[test]
fn jpeg_has_eoi_marker() {
    let jpeg_data = build_minimal_jpeg(64, 64);
    let tmp = std::env::temp_dir().join("test_eoi.jpg");
    fs::write(&tmp, &jpeg_data).unwrap();

    let analysis = image_analyzer::analyzer::jpeg_parser::analyze_jpeg(tmp.to_str().unwrap())
        .expect("Should parse");

    let eoi = analysis
        .structure
        .iter()
        .find(|b| b.name == "EOI")
        .expect("JPEG should have EOI marker");

    assert!(eoi.decoded_info.is_some());

    fs::remove_file(&tmp).ok();
}

#[test]
fn jpeg_has_sof0_with_dimensions() {
    let jpeg_data = build_minimal_jpeg(800, 600);
    let tmp = std::env::temp_dir().join("test_sof0.jpg");
    fs::write(&tmp, &jpeg_data).unwrap();

    let analysis = image_analyzer::analyzer::jpeg_parser::analyze_jpeg(tmp.to_str().unwrap())
        .expect("Should parse");

    let sof0 = analysis
        .structure
        .iter()
        .find(|b| b.name.contains("SOF0"))
        .expect("JPEG should have SOF0 marker");

    assert!(sof0.decoded_info.is_some());
    let info = sof0.decoded_info.as_ref().unwrap();
    assert!(info.contains("Width: 800"));
    assert!(info.contains("Height: 600"));
    assert!(info.contains("Bit Depth: 8"));

    assert_eq!(analysis.width, 800);
    assert_eq!(analysis.height, 600);

    fs::remove_file(&tmp).ok();
}

#[test]
fn jpeg_has_app0_jfif() {
    let jpeg_data = build_minimal_jpeg(64, 64);
    let tmp = std::env::temp_dir().join("test_jfif.jpg");
    fs::write(&tmp, &jpeg_data).unwrap();

    let analysis = image_analyzer::analyzer::jpeg_parser::analyze_jpeg(tmp.to_str().unwrap())
        .expect("Should parse");

    let app0 = analysis
        .structure
        .iter()
        .find(|b| b.name.contains("APP0"))
        .expect("JPEG should have APP0 marker");

    assert!(app0.decoded_info.is_some());
    let info = app0.decoded_info.as_ref().unwrap();
    assert!(info.contains("JFIF"));

    fs::remove_file(&tmp).ok();
}

#[test]
fn jpeg_structure_blocks_have_offsets_and_lengths() {
    let jpeg_data = build_minimal_jpeg(64, 64);
    let tmp = std::env::temp_dir().join("test_offsets.jpg");
    fs::write(&tmp, &jpeg_data).unwrap();

    let analysis = image_analyzer::analyzer::jpeg_parser::analyze_jpeg(tmp.to_str().unwrap())
        .expect("Should parse");

    for block in &analysis.structure {
        assert!(block.length > 0, "Block length should be positive");
        assert!(!block.name.is_empty(), "Block name should not be empty");
    }

    fs::remove_file(&tmp).ok();
}

#[test]
fn first_block_is_soi() {
    let jpeg_data = build_minimal_jpeg(64, 64);
    let tmp = std::env::temp_dir().join("test_first.jpg");
    fs::write(&tmp, &jpeg_data).unwrap();

    let analysis = image_analyzer::analyzer::jpeg_parser::analyze_jpeg(tmp.to_str().unwrap())
        .expect("Should parse");

    assert_eq!(
        analysis.structure[0].name, "SOI",
        "First block should be SOI"
    );

    fs::remove_file(&tmp).ok();
}

#[test]
fn jpeg_format_is_correct() {
    let jpeg_data = build_minimal_jpeg(64, 64);
    let tmp = std::env::temp_dir().join("test_format.jpg");
    fs::write(&tmp, &jpeg_data).unwrap();

    let analysis = image_analyzer::analyzer::jpeg_parser::analyze_jpeg(tmp.to_str().unwrap())
        .expect("Should parse");

    match analysis.format {
        image_analyzer::types::ImageFormat::Jpeg => {}
        _ => panic!("Format should be Jpeg"),
    }

    fs::remove_file(&tmp).ok();
}

#[test]
fn jpeg_with_comment() {
    let mut data = Vec::new();
    data.extend_from_slice(&[0xFF, 0xD8]); // SOI

    // COM marker
    let comment = b"Test JPEG comment";
    write_jpeg_segment(&mut data, 0xFE, comment);

    // SOF0
    let sof0: Vec<u8> = vec![0x08, 0x00, 0x10, 0x00, 0x10, 0x01, 0x01, 0x11, 0x00];
    write_jpeg_segment(&mut data, 0xC0, &sof0);

    // SOS
    let sos: Vec<u8> = vec![0x01, 0x01, 0x00, 0x00, 0x3F, 0x00];
    write_jpeg_segment(&mut data, 0xDA, &sos);
    data.push(0x00);
    data.push(0xFF);
    data.push(0x00);

    // EOI
    data.extend_from_slice(&[0xFF, 0xD9]);

    let tmp = std::env::temp_dir().join("test_comment.jpg");
    fs::write(&tmp, &data).unwrap();

    let analysis = image_analyzer::analyzer::jpeg_parser::analyze_jpeg(tmp.to_str().unwrap())
        .expect("Should parse");

    let com = analysis
        .structure
        .iter()
        .find(|b| b.name == "COM")
        .expect("JPEG should have COM marker");

    assert!(com.decoded_info.is_some());
    let info = com.decoded_info.as_ref().unwrap();
    assert!(info.contains("Test JPEG comment"));

    fs::remove_file(&tmp).ok();
}

#[test]
fn jpeg_with_exif_metadata() {
    let jpeg_data = build_jpeg_with_exif(640, 480);
    let tmp = std::env::temp_dir().join("test_exif.jpg");
    fs::write(&tmp, &jpeg_data).unwrap();

    let analysis = image_analyzer::analyzer::jpeg_parser::analyze_jpeg(tmp.to_str().unwrap())
        .expect("Should parse JPEG with EXIF");

    // Check that EXIF metadata was extracted
    let exif_entries: Vec<_> = analysis
        .metadata
        .iter()
        .filter(|m| m.standard == "EXIF")
        .collect();

    assert!(
        !exif_entries.is_empty(),
        "Should have EXIF metadata entries"
    );

    // Check for ImageWidth or ImageHeight (numeric tags that should parse reliably)
    let width_entries: Vec<_> = exif_entries
        .iter()
        .filter(|m| m.tag_name.contains("ImageWidth") || m.tag_name.contains("0x0100"))
        .collect();
    assert!(
        !width_entries.is_empty(),
        "Should have ImageWidth EXIF entry, found: {:?}",
        exif_entries
    );

    fs::remove_file(&tmp).ok();
}

#[test]
fn jpeg_with_xmp_metadata() {
    let jpeg_data = build_jpeg_with_xmp(640, 480);
    let tmp = std::env::temp_dir().join("test_xmp.jpg");
    fs::write(&tmp, &jpeg_data).unwrap();

    let analysis = image_analyzer::analyzer::jpeg_parser::analyze_jpeg(tmp.to_str().unwrap())
        .expect("Should parse JPEG with XMP");

    let xmp_entries: Vec<_> = analysis
        .metadata
        .iter()
        .filter(|m| m.standard == "XMP")
        .collect();

    assert!(!xmp_entries.is_empty(), "Should have XMP metadata entries");

    // Check that CreatorTool was extracted
    let creators: Vec<_> = xmp_entries
        .iter()
        .filter(|m| m.tag_name == "CreatorTool")
        .collect();
    assert!(
        !creators.is_empty(),
        "Should have CreatorTool XMP entry, found: {:?}",
        xmp_entries
    );
    assert_eq!(creators[0].tag_value, "Test Camera");

    // Check Rating
    let ratings: Vec<_> = xmp_entries
        .iter()
        .filter(|m| m.tag_name == "Rating")
        .collect();
    assert!(!ratings.is_empty(), "Should have Rating XMP entry");

    fs::remove_file(&tmp).ok();
}

#[test]
fn jpeg_with_iptc_metadata() {
    let jpeg_data = build_jpeg_with_iptc(640, 480);
    let tmp = std::env::temp_dir().join("test_iptc.jpg");
    fs::write(&tmp, &jpeg_data).unwrap();

    let analysis = image_analyzer::analyzer::jpeg_parser::analyze_jpeg(tmp.to_str().unwrap())
        .expect("Should parse JPEG with IPTC");

    let iptc_entries: Vec<_> = analysis
        .metadata
        .iter()
        .filter(|m| m.standard == "IPTC")
        .collect();

    assert!(
        !iptc_entries.is_empty(),
        "Should have IPTC metadata entries, found: {:?}",
        analysis.metadata
    );

    // Check ObjectName
    let names: Vec<_> = iptc_entries
        .iter()
        .filter(|m| m.tag_name == "ObjectName")
        .collect();
    assert!(!names.is_empty(), "Should have ObjectName IPTC entry");
    assert_eq!(names[0].tag_value, "Test Image");

    // Check Caption
    let captions: Vec<_> = iptc_entries
        .iter()
        .filter(|m| m.tag_name == "Caption")
        .collect();
    assert!(!captions.is_empty(), "Should have Caption IPTC entry");
    assert!(
        captions[0].tag_value.contains("test caption"),
        "Caption should contain 'test caption'"
    );

    // Check By-line
    let bylines: Vec<_> = iptc_entries
        .iter()
        .filter(|m| m.tag_name == "By-line")
        .collect();
    assert!(!bylines.is_empty(), "Should have By-line IPTC entry");

    fs::remove_file(&tmp).ok();
}

#[test]
fn rejects_file_with_invalid_signature() {
    let tmp = std::env::temp_dir().join("test_invalid_sig.jpg");
    let data = vec![0x00u8; 32];
    fs::write(&tmp, &data).unwrap();

    let result = image_analyzer::analyzer::jpeg_parser::analyze_jpeg(tmp.to_str().unwrap());
    assert!(result.is_err(), "Should reject invalid JPEG signature");
    let err = result.unwrap_err();
    assert!(
        err.contains("signature") || err.contains("Invalid"),
        "Error should mention signature: {}",
        err
    );

    fs::remove_file(&tmp).ok();
}

#[test]
fn rejects_empty_file() {
    let tmp = std::env::temp_dir().join("test_empty.jpg");
    fs::write(&tmp, b"").unwrap();

    let result = image_analyzer::analyzer::jpeg_parser::analyze_jpeg(tmp.to_str().unwrap());
    assert!(result.is_err(), "Should reject empty file");

    fs::remove_file(&tmp).ok();
}

#[test]
fn rejects_nonexistent_file() {
    let result = image_analyzer::analyzer::jpeg_parser::analyze_jpeg("/nonexistent/path/file.jpg");
    assert!(result.is_err(), "Should reject nonexistent file");
}

#[test]
fn jpeg_with_progressive_sof2() {
    let mut data = Vec::new();
    data.extend_from_slice(&[0xFF, 0xD8]); // SOI

    // APP0
    let jfif: Vec<u8> = vec![
        b'J', b'F', b'I', b'F', 0x00, 0x01, 0x02, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00,
    ];
    write_jpeg_segment(&mut data, 0xE0, &jfif);

    // SOF2 (Progressive)
    let sof2: Vec<u8> = vec![
        0x08, 0x00, 0x20, 0x00, 0x20, 0x03, 0x01, 0x22, 0x00, 0x02, 0x11, 0x00, 0x03, 0x11, 0x00,
    ];
    write_jpeg_segment(&mut data, 0xC2, &sof2);

    // SOS
    let sos: Vec<u8> = vec![0x03, 0x01, 0x00, 0x02, 0x11, 0x03, 0x11, 0x00, 0x3F, 0x00];
    write_jpeg_segment(&mut data, 0xDA, &sos);
    data.push(0x00);
    data.push(0xFF);
    data.push(0x00);

    // EOI
    data.extend_from_slice(&[0xFF, 0xD9]);

    let tmp = std::env::temp_dir().join("test_progressive.jpg");
    fs::write(&tmp, &data).unwrap();

    let analysis = image_analyzer::analyzer::jpeg_parser::analyze_jpeg(tmp.to_str().unwrap())
        .expect("Should parse progressive JPEG");

    let sof2_block = analysis
        .structure
        .iter()
        .find(|b| b.name.contains("SOF2") || b.name.contains("Progressive"))
        .expect("Should have SOF2 marker");

    assert!(
        sof2_block.name.contains("Progressive"),
        "Block name should mention progressive: {}",
        sof2_block.name
    );

    fs::remove_file(&tmp).ok();
}

#[test]
fn jpeg_with_adobe_app14() {
    let mut data = Vec::new();
    data.extend_from_slice(&[0xFF, 0xD8]); // SOI

    // APP14 (Adobe)
    let adobe: Vec<u8> = vec![
        b'A', b'd', b'o', b'b', b'e', // "Adobe"
        0x00, 0x64, // Version 100
        0x00, 0x00, // Flags0
        0x00, 0x00, // Flags1
        0x01, // Color transform = YCbCr
    ];
    write_jpeg_segment(&mut data, 0xEE, &adobe);

    // SOF0
    let sof0: Vec<u8> = vec![
        0x08, 0x00, 0x10, 0x00, 0x10, 0x03, 0x01, 0x11, 0x00, 0x02, 0x11, 0x00, 0x03, 0x11, 0x00,
    ];
    write_jpeg_segment(&mut data, 0xC0, &sof0);

    // SOS
    let sos: Vec<u8> = vec![0x03, 0x01, 0x00, 0x02, 0x11, 0x03, 0x11, 0x00, 0x3F, 0x00];
    write_jpeg_segment(&mut data, 0xDA, &sos);
    data.push(0x00);
    data.push(0xFF);
    data.push(0x00);

    // EOI
    data.extend_from_slice(&[0xFF, 0xD9]);

    let tmp = std::env::temp_dir().join("test_adobe.jpg");
    fs::write(&tmp, &data).unwrap();

    let analysis = image_analyzer::analyzer::jpeg_parser::analyze_jpeg(tmp.to_str().unwrap())
        .expect("Should parse JPEG with Adobe APP14");

    let app14 = analysis
        .structure
        .iter()
        .find(|b| b.name.contains("APP14"))
        .expect("Should have APP14 marker");

    assert!(app14.decoded_info.is_some());
    let info = app14.decoded_info.as_ref().unwrap();
    assert!(info.contains("Adobe"));

    fs::remove_file(&tmp).ok();
}

#[test]
fn jpeg_marker_count_matches_structure() {
    let jpeg_data = build_minimal_jpeg(64, 64);
    let tmp = std::env::temp_dir().join("test_count.jpg");
    fs::write(&tmp, &jpeg_data).unwrap();

    let analysis = image_analyzer::analyzer::jpeg_parser::analyze_jpeg(tmp.to_str().unwrap())
        .expect("Should parse");

    let mut counts: HashMap<String, u32> = HashMap::new();
    for block in &analysis.structure {
        *counts.entry(block.name.clone()).or_insert(0) += 1;
    }

    assert_eq!(
        *counts.get("SOI").unwrap_or(&0),
        1,
        "Should have exactly 1 SOI"
    );
    assert_eq!(
        *counts.get("EOI").unwrap_or(&0),
        1,
        "Should have exactly 1 EOI"
    );
    assert!(
        *counts.get("APP0 (JFIF)").unwrap_or(&0) >= 1,
        "Should have at least 1 APP0"
    );

    fs::remove_file(&tmp).ok();
}

#[test]
fn jpeg_with_icc_profile_app2() {
    let mut data = Vec::new();
    data.extend_from_slice(&[0xFF, 0xD8]); // SOI

    // APP2 (ICC_PROFILE part 1 of 1)
    let mut icc = Vec::new();
    icc.extend_from_slice(b"ICC_PROFILE\0\0\0"); // Identifier
    icc.push(0x01); // Sequence number
    icc.push(0x01); // Total count
    icc.extend_from_slice(&[0x00u8; 16]); // Minimal ICC header padding
    write_jpeg_segment(&mut data, 0xE2, &icc);

    // SOF0
    let sof0: Vec<u8> = vec![0x08, 0x00, 0x10, 0x00, 0x10, 0x01, 0x01, 0x11, 0x00];
    write_jpeg_segment(&mut data, 0xC0, &sof0);

    // SOS
    let sos: Vec<u8> = vec![0x01, 0x01, 0x00, 0x00, 0x3F, 0x00];
    write_jpeg_segment(&mut data, 0xDA, &sos);
    data.push(0x00);
    data.push(0xFF);
    data.push(0x00);

    // EOI
    data.extend_from_slice(&[0xFF, 0xD9]);

    let tmp = std::env::temp_dir().join("test_icc.jpg");
    fs::write(&tmp, &data).unwrap();

    let analysis = image_analyzer::analyzer::jpeg_parser::analyze_jpeg(tmp.to_str().unwrap())
        .expect("Should parse JPEG with ICC");

    let app2 = analysis
        .structure
        .iter()
        .find(|b| b.name.contains("APP2"))
        .expect("Should have APP2 marker");

    assert!(app2.decoded_info.is_some());
    let info = app2.decoded_info.as_ref().unwrap();
    assert!(
        info.contains("ICC"),
        "APP2 should mention ICC profile: {}",
        info
    );

    fs::remove_file(&tmp).ok();
}

#[test]
fn sof_component_children_present() {
    let jpeg_data = build_minimal_jpeg(64, 64);
    let tmp = std::env::temp_dir().join("test_comps.jpg");
    fs::write(&tmp, &jpeg_data).unwrap();

    let analysis = image_analyzer::analyzer::jpeg_parser::analyze_jpeg(tmp.to_str().unwrap())
        .expect("Should parse");

    let sof_block = analysis
        .structure
        .iter()
        .find(|b| b.name.contains("SOF0"))
        .expect("Should have SOF0");

    assert!(
        !sof_block.children.is_empty(),
        "SOF0 should have component children"
    );

    // Check for Y component
    let y_comp = sof_block
        .children
        .iter()
        .find(|c| c.name == "Component 1")
        .expect("Should have Component 1 (Y)");

    assert!(y_comp.decoded_info.is_some());
    let info = y_comp.decoded_info.as_ref().unwrap();
    assert!(info.contains("Y (Luma)"));

    fs::remove_file(&tmp).ok();
}

#[test]
fn parses_jpeg_with_multiple_app_markers() {
    let mut data = Vec::new();
    data.extend_from_slice(&[0xFF, 0xD8]); // SOI

    // APP0 (JFIF)
    let jfif: Vec<u8> = vec![
        b'J', b'F', b'I', b'F', 0x00, 0x01, 0x02, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00,
    ];
    write_jpeg_segment(&mut data, 0xE0, &jfif);

    // APP14 (Adobe)
    let adobe: Vec<u8> = vec![
        b'A', b'd', b'o', b'b', b'e', 0x00, 0x64, 0x00, 0x00, 0x00, 0x00, 0x01,
    ];
    write_jpeg_segment(&mut data, 0xEE, &adobe);

    // COM
    write_jpeg_segment(&mut data, 0xFE, b"Multi-marker test");

    // SOF0
    let sof0: Vec<u8> = vec![0x08, 0x00, 0x10, 0x00, 0x10, 0x01, 0x01, 0x11, 0x00];
    write_jpeg_segment(&mut data, 0xC0, &sof0);

    // SOS
    let sos: Vec<u8> = vec![0x01, 0x01, 0x00, 0x00, 0x3F, 0x00];
    write_jpeg_segment(&mut data, 0xDA, &sos);
    data.push(0x00);
    data.push(0xFF);
    data.push(0x00);

    // EOI
    data.extend_from_slice(&[0xFF, 0xD9]);

    let tmp = std::env::temp_dir().join("test_multi.jpg");
    fs::write(&tmp, &data).unwrap();

    let analysis = image_analyzer::analyzer::jpeg_parser::analyze_jpeg(tmp.to_str().unwrap())
        .expect("Should parse");

    assert!(
        analysis.structure.iter().any(|b| b.name.contains("APP0")),
        "Should have APP0"
    );
    assert!(
        analysis.structure.iter().any(|b| b.name.contains("APP14")),
        "Should have APP14"
    );
    assert!(
        analysis.structure.iter().any(|b| b.name == "COM"),
        "Should have COM"
    );

    fs::remove_file(&tmp).ok();
}

#[test]
fn file_size_matches_input() {
    let jpeg_data = build_minimal_jpeg(64, 64);
    let tmp = std::env::temp_dir().join("test_filesize.jpg");
    fs::write(&tmp, &jpeg_data).unwrap();

    let analysis = image_analyzer::analyzer::jpeg_parser::analyze_jpeg(tmp.to_str().unwrap())
        .expect("Should parse");

    assert_eq!(
        analysis.file_size,
        jpeg_data.len() as u64,
        "File size should match input"
    );

    fs::remove_file(&tmp).ok();
}

#[test]
fn bit_depth_from_sof() {
    // Build JPEG with 12-bit precision (SOF with precision = 12)
    let mut data = Vec::new();
    data.extend_from_slice(&[0xFF, 0xD8]);

    // SOF0 with 12-bit precision
    let sof0: Vec<u8> = vec![
        0x0C, // 12-bit precision
        0x00, 0x10, // height = 16
        0x00, 0x10, // width = 16
        0x01, // 1 component
        0x01, 0x11, 0x00,
    ];
    write_jpeg_segment(&mut data, 0xC0, &sof0);

    // SOS
    let sos: Vec<u8> = vec![0x01, 0x01, 0x00, 0x00, 0x3F, 0x00];
    write_jpeg_segment(&mut data, 0xDA, &sos);
    data.push(0x00);
    data.push(0xFF);
    data.push(0x00);

    // EOI
    data.extend_from_slice(&[0xFF, 0xD9]);

    let tmp = std::env::temp_dir().join("test_12bit.jpg");
    fs::write(&tmp, &data).unwrap();

    let analysis = image_analyzer::analyzer::jpeg_parser::analyze_jpeg(tmp.to_str().unwrap())
        .expect("Should parse");

    assert_eq!(analysis.bit_depth, 12, "Bit depth should be 12");

    fs::remove_file(&tmp).ok();
}
