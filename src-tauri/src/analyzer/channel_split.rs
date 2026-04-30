use crate::types::ChannelData;

pub fn split_channels(_data: &[u8], _width: u32, _height: u32) -> Result<ChannelData, String> {
    Err("Channel split not yet implemented".to_string())
}
