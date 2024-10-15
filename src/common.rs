use crate::bindings::{
    IMPDeviceID_DEV_ID_ENC as DEV_ID_ENC, IMPDeviceID_DEV_ID_FS as DEV_ID_FS,
    IMPEncoderRcMode_ENC_RC_MODE_CBR as ENC_RC_MODE_CBR, IMPFSChnAttr, IMPFSChnCrop,
    IMPFSChnScaler, IMPFSChnType_FS_EXT_CHANNEL as FS_EXT_CHANNEL,
    IMPFSChnType_FS_PHY_CHANNEL as FS_PHY_CHANNEL, IMPPayloadType_PT_H264 as PT_H264,
    IMPPixelFormat_PIX_FMT_HSV as PIX_FMT_HSV, IMPPixelFormat_PIX_FMT_NV12 as PIX_FMT_NV12,
    IMPPixelFormat_PIX_FMT_RGBA as PIX_FMT_RGBA, IMPSensorInfo,
};

use crate::bindings::{
    IMPCell, IMPPayloadType, IMPSensorControlBusType,
    IMPSensorControlBusType_TX_SENSOR_CONTROL_INTERFACE_I2C as TX_SENSOR_CONTROL_INTERFACE_I2C,
};
const SENSOR_FRAME_RATE_NUM: i32 = 25;
const SENSOR_FRAME_RATE_DEN: i32 = 1;
use libc::c_int;

#[derive(Debug, Default)]
struct SensorConfig {
    pub name: &'static str,
    pub cubs_type: IMPSensorControlBusType,
    pub i2c_addr: u8,
    pub i2c_adapter_id: u8,
    pub width: i32,
    pub height: i32,
    pub chn_enabled: [usize; 4], //chn0,chn1,chn2,chn3
    pub crop_enabled: usize,
    pub common: SensorCommon,
}
#[cfg(feature = "SENSOR_GC2063")]
const SENSOR_CONFIG: SensorConfig = SensorConfig {
    name: "gc2053",
    cubs_type: TX_SENSOR_CONTROL_INTERFACE_I2C,
    i2c_addr: 0x37,
    i2c_adapter_id: 1,
    width: 1920,
    height: 1080,
    chn_enabled: [1, 0, 0, 0], //chn0,chn1,chn2,chn3
    crop_enabled: 0,
    common: SensorCommon::new(),
};

#[cfg(feature = "SENSOR_GC2053")]
const SENSOR_CONFIG: SensorConfig = SensorConfig {
    name: "gc2053",
    cubs_type: TX_SENSOR_CONTROL_INTERFACE_I2C,
    i2c_addr: 0x37,
    i2c_adapter_id: 0,
    width: 1920,
    height: 1080,
    chn_enabled: [1, 0, 0, 0],
    crop_enabled: 0,
    common: SensorCommon::new(),
};

#[derive(Debug, Default)]
struct SensorCommon {
    width_second: i32,
    height_second: i32,
    width_third: i32,
    height_third: i32,
}
impl SensorCommon {
    const fn new() -> Self {
        Self {
            width_second: 640,
            height_second: 360,
            width_third: 1280,
            height_third: 720,
        }
    }
}

const BITRATE_720P_Kbs: u32 = 1000;

const NR_FRAMES_TO_SAVE: u32 = 200;
const NR_JPEG_TO_SAVE: u32 = 20;
const STREAM_BUFFER_SIZE: u32 = 1 * 1024 * 1024;

const ENC_VIDEO_CHANNEL: u32 = 0;
const ENC_JPEG_CHANNEL: u32 = 1;

const STREAM_FILE_PATH_PREFIX: &str = "/tmp";
const SNAP_FILE_PATH_PREFIX: &str = "/tmp";

pub const OSD_REGION_WIDTH: u32 = 16;
pub const OSD_REGION_HEIGHT: u32 = 34;
pub const OSD_REGION_WIDTH_SEC: u32 = 8;
pub const OSD_REGION_HEIGHT_SEC: u32 = 18;

const SLEEP_TIME: u64 = 1; // 秒

const FS_CHN_NUM: usize = 4; // MIN 1, MAX 4
const IVS_CHN_ID: usize = 2;

const CH0_INDEX: i32 = 0;
const CH1_INDEX: i32 = 1;
const CH2_INDEX: i32 = 2;
const CH3_INDEX: i32 = 3; // ext chn

const CHN_ENABLE: usize = 1;
const CHN_DISABLE: usize = 0;

#[derive(Debug, Default)]
pub struct ChnConf {
    pub index: i32,    // 0 表示主通道, 1 表示次通道
    pub enable: usize, // 通道是否启用
    pub payload_type: IMPPayloadType,
    pub fs_chn_attr: IMPFSChnAttr,
    pub framesource_chn: IMPCell,
    pub imp_encoder: IMPCell,
}
const CHN_NUM: usize = unsafe { CHN.len() };
const TAG: &str = "Sample-Common";
const S_RC_METHOD: i32 = ENC_RC_MODE_CBR as i32;
static direct_swicth: i32 = 0;
static GOSD_ENABLE: i32 = 0; /* 1: ipu osd, 2: isp osd, 3: ipu osd and isp osd */

#[cfg(feature = "SHOW_FRM_BITRATE")]
mod show_frm_bitrate {
    const FRM_BIT_RATE_TIME: i32 = 2;
    const STREAM_TYPE_NUM: usize = 3;
    static frmrate_sp: [i32; STREAM_TYPE_NUM] = [0i32; STREAM_TYPE_NUM];
    static statime_sp: [i32; STREAM_TYPE_NUM] = [0i32; STREAM_TYPE_NUM];
    static bitrate_sp: [i32; STREAM_TYPE_NUM] = [0i32; STREAM_TYPE_NUM];
}

pub static mut CHN: [ChnConf; FS_CHN_NUM] = [
    ChnConf {
        index: CH0_INDEX,
        enable: SENSOR_CONFIG.chn_enabled[0],
        payload_type: PT_H264,
        fs_chn_attr: IMPFSChnAttr {
            pixFmt: PIX_FMT_NV12,
            outFrmRateNum: SENSOR_FRAME_RATE_NUM,
            outFrmRateDen: SENSOR_FRAME_RATE_DEN,
            nrVBs: 2,
            type_: FS_PHY_CHANNEL,
            crop: IMPFSChnCrop {
                enable: 1,
                top: 0,
                left: 0,
                width: SENSOR_CONFIG.width,
                height: SENSOR_CONFIG.height,
            },
            scaler: IMPFSChnScaler {
                enable: 1,
                outwidth: SENSOR_CONFIG.width,
                outheight: SENSOR_CONFIG.height,
            },
            picWidth: SENSOR_CONFIG.width,
            picHeight: SENSOR_CONFIG.height,
            mirr_enable: 0,
            fcrop: IMPFSChnCrop {
                enable: 0,
                top: 0,
                left: 0,
                width: 0,
                height: 0,
            },
        },
        framesource_chn: IMPCell {
            deviceID: DEV_ID_FS,
            groupID: CH0_INDEX,
            outputID: 0,
        },
        imp_encoder: IMPCell {
            deviceID: DEV_ID_ENC,
            groupID: CH0_INDEX,
            outputID: 0,
        },
    },
    ChnConf {
        index: CH1_INDEX,
        enable: SENSOR_CONFIG.chn_enabled[1],
        payload_type: PT_H264,
        fs_chn_attr: IMPFSChnAttr {
            pixFmt: PIX_FMT_NV12,
            outFrmRateNum: SENSOR_FRAME_RATE_NUM,
            outFrmRateDen: SENSOR_FRAME_RATE_DEN,
            nrVBs: 2,
            type_: FS_PHY_CHANNEL,
            crop: IMPFSChnCrop {
                enable: 1,
                top: 0,
                left: 0,
                width: SENSOR_CONFIG.width,
                height: SENSOR_CONFIG.height,
            },
            scaler: IMPFSChnScaler {
                enable: 1,
                outwidth: SENSOR_CONFIG.common.width_second,
                outheight: SENSOR_CONFIG.common.height_second,
            },
            picWidth: SENSOR_CONFIG.common.width_second,
            picHeight: SENSOR_CONFIG.common.width_second,
            mirr_enable: 0,
            fcrop: IMPFSChnCrop {
                enable: 0,
                top: 0,
                left: 0,
                width: 0,
                height: 0,
            },
        },
        framesource_chn: IMPCell {
            deviceID: DEV_ID_FS,
            groupID: CH1_INDEX,
            outputID: 0,
        },
        imp_encoder: IMPCell {
            deviceID: DEV_ID_ENC,
            groupID: CH1_INDEX,
            outputID: 0,
        },
    },
    ChnConf {
        index: CH2_INDEX,
        enable: SENSOR_CONFIG.chn_enabled[2],
        payload_type: PT_H264,
        fs_chn_attr: IMPFSChnAttr {
            pixFmt: PIX_FMT_NV12,
            outFrmRateNum: SENSOR_FRAME_RATE_NUM,
            outFrmRateDen: SENSOR_FRAME_RATE_DEN,
            nrVBs: 2,
            type_: FS_PHY_CHANNEL,
            crop: IMPFSChnCrop {
                enable: 1,
                top: 0,
                left: 0,
                width: SENSOR_CONFIG.common.width_second,
                height: SENSOR_CONFIG.common.height_second,
            },
            scaler: IMPFSChnScaler {
                enable: 1,
                outwidth: SENSOR_CONFIG.common.width_second,
                outheight: SENSOR_CONFIG.common.height_second,
            },
            picWidth: SENSOR_CONFIG.common.width_second,
            picHeight: SENSOR_CONFIG.common.height_second,
            mirr_enable: 0,
            fcrop: IMPFSChnCrop {
                enable: 0,
                top: 0,
                left: 0,
                width: 0,
                height: 0,
            },
        },
        framesource_chn: IMPCell {
            deviceID: DEV_ID_FS,
            groupID: CH2_INDEX,
            outputID: 0,
        },
        imp_encoder: IMPCell {
            deviceID: DEV_ID_ENC,
            groupID: CH2_INDEX,
            outputID: 0,
        },
    },
    ChnConf {
        index: CH3_INDEX,
        enable: SENSOR_CONFIG.chn_enabled[3],
        payload_type: PT_H264,
        fs_chn_attr: IMPFSChnAttr {
            pixFmt: PIX_FMT_NV12,
            outFrmRateNum: SENSOR_FRAME_RATE_NUM,
            outFrmRateDen: SENSOR_FRAME_RATE_DEN,
            nrVBs: 2,
            type_: FS_PHY_CHANNEL,
            crop: IMPFSChnCrop {
                enable: 0,
                top: 0,
                left: 0,
                width: SENSOR_CONFIG.width,
                height: SENSOR_CONFIG.height,
            },
            scaler: IMPFSChnScaler {
                enable: 1,
                outwidth: SENSOR_CONFIG.width,
                outheight: SENSOR_CONFIG.height,
            },
            picWidth: SENSOR_CONFIG.width,
            picHeight: SENSOR_CONFIG.height,
            mirr_enable: 0,
            fcrop: IMPFSChnCrop {
                enable: 0,
                top: 0,
                left: 0,
                width: 0,
                height: 0,
            },
        },
        framesource_chn: IMPCell {
            deviceID: DEV_ID_FS,
            groupID: CH3_INDEX,
            outputID: 0,
        },
        imp_encoder: IMPCell {
            deviceID: DEV_ID_ENC,
            groupID: CH3_INDEX,
            outputID: 0,
        },
    },
];

impl IMPFSChnCrop {
    const fn new() -> Self {
        IMPFSChnCrop {
            enable: 0,
            top: 0,
            left: 0,
            width: 0,
            height: 0,
        }
    }
}

impl IMPFSChnScaler {
    const fn new() -> Self {
        IMPFSChnScaler {
            enable: 0,
            outwidth: 0,
            outheight: 0,
        }
    }
}

impl IMPFSChnAttr {
    const fn new() -> Self {
        IMPFSChnAttr {
            picHeight: 0,
            picWidth: 0,
            pixFmt: 0,
            crop: IMPFSChnCrop::new(),
            scaler: IMPFSChnScaler::new(),
            outFrmRateDen: 0,
            outFrmRateNum: 0,
            nrVBs: 0,
            type_: 0,
            mirr_enable: 0,
            fcrop: IMPFSChnCrop::new(),
        }
    }
}

impl IMPCell {
    const fn new() -> Self {
        IMPCell {
            deviceID: 0,
            groupID: 0,
            outputID: 0,
        }
    }
}

impl ChnConf {
    const fn new() -> Self {
        ChnConf {
            index: 0,
            enable: 0,
            payload_type: 0,
            fs_chn_attr: IMPFSChnAttr::new(),
            framesource_chn: IMPCell::new(),
            imp_encoder: IMPCell::new(),
        }
    }
}

pub const CHN_EXT_HSV: [ChnConf; 1] = [ChnConf {
    fs_chn_attr: IMPFSChnAttr {
        pixFmt: PIX_FMT_HSV,
        outFrmRateNum: SENSOR_FRAME_RATE_NUM,
        outFrmRateDen: SENSOR_FRAME_RATE_DEN,
        nrVBs: 2,
        type_: FS_EXT_CHANNEL,
        crop: IMPFSChnCrop {
            enable: 0,
            top: 0,
            left: 0,
            width: SENSOR_CONFIG.width,
            height: SENSOR_CONFIG.height,
        },
        scaler: IMPFSChnScaler {
            enable: 1,
            outwidth: SENSOR_CONFIG.common.width_second,
            outheight: SENSOR_CONFIG.common.height_second,
        },
        picWidth: SENSOR_CONFIG.common.width_second,
        picHeight: SENSOR_CONFIG.common.height_second,
        ..IMPFSChnAttr::new()
    },
    ..ChnConf::new()
}];

// 初始化 chn_ext_rgba
pub const CHN_EXT_RGBA: [ChnConf; 1] = [ChnConf {
    fs_chn_attr: IMPFSChnAttr {
        pixFmt: PIX_FMT_RGBA,
        outFrmRateNum: SENSOR_FRAME_RATE_NUM,
        outFrmRateDen: SENSOR_FRAME_RATE_DEN,
        nrVBs: 2,
        type_: FS_EXT_CHANNEL,
        crop: IMPFSChnCrop {
            enable: 0,
            top: 0,
            left: 0,
            width: SENSOR_CONFIG.width,
            height: SENSOR_CONFIG.height,
        },
        scaler: IMPFSChnScaler {
            enable: 1,
            outwidth: SENSOR_CONFIG.common.width_second,
            outheight: SENSOR_CONFIG.common.height_second,
        },
        picWidth: SENSOR_CONFIG.common.width_second,
        picHeight: SENSOR_CONFIG.common.height_second,
        ..IMPFSChnAttr::new()
    },
    ..ChnConf::new()
}];

extern "C" {
    pub fn IMP_Encoder_SetPoolSize(size: c_int) -> c_int;
    pub fn IMP_OSD_SetPoolSize(size: c_int) -> c_int;
}

pub fn sample_system_init() -> i32 {
    let mut ret: i32 = 0;

    /* isp osd and ipu osd buffer size set */
    if (1 == GOSD_ENABLE) {
        unsafe {
            IMP_OSD_SetPoolSize(512 * 1024);
        }
    }

    ret
}
