use crate::{
    bindings::{
        strerror, IMPAudioPalyloadType, IMPCell,
        IMPDeviceID::{self, *},
        IMPEncoderAttr, IMPEncoderCHNAttr, IMPEncoderCHNStat, IMPEncoderRcAttr,
        IMPEncoderRcMode::{self, *},
        IMPEncoderStream, IMPFSChnAttr, IMPFSChnCrop, IMPFSChnScaler,
        IMPFSChnType::{self, *},
        IMPISPRunningMode::*,
        IMPOSDGrpRgnAttr, IMPOSDRgnAttr,
        IMPOsdColour::{self, *},
        IMPOsdRgnType::{self, *},
        IMPPayloadType::{self, *},
        IMPPixelFormat::{self, *},
        IMPRgnHandle,
        IMPSensorControlBusType::{self, *},
        IMPSensorInfo,
        IMPSkipType::{self, *},
        IMP_Encoder_CreateChn, IMP_Encoder_DestroyChn, IMP_Encoder_GetFd, IMP_Encoder_GetStream,
        IMP_Encoder_PollingStream, IMP_Encoder_Query, IMP_Encoder_RegisterChn,
        IMP_Encoder_ReleaseStream, IMP_Encoder_StartRecvPic, IMP_Encoder_StopRecvPic,
        IMP_Encoder_UnRegisterChn, IMP_FrameSource_CreateChn, IMP_FrameSource_DestroyChn,
        IMP_FrameSource_DisableChn, IMP_FrameSource_EnableChn, IMP_FrameSource_SetChnAttr,
        IMP_ISP_AddSensor, IMP_ISP_Close, IMP_ISP_DelSensor, IMP_ISP_DisableSensor,
        IMP_ISP_DisableTuning, IMP_ISP_EnableSensor, IMP_ISP_EnableTuning, IMP_ISP_Open,
        IMP_ISP_Tuning_SetBrightness, IMP_ISP_Tuning_SetContrast, IMP_ISP_Tuning_SetISPRunningMode,
        IMP_ISP_Tuning_SetOsdPoolSize, IMP_ISP_Tuning_SetSaturation, IMP_ISP_Tuning_SetSharpness,
        IMP_OSD_CreateRgn, IMP_OSD_DestroyGroup, IMP_OSD_DestroyRgn, IMP_OSD_GetGrpRgnAttr,
        IMP_OSD_RegisterRgn, IMP_OSD_SetGrpRgnAttr, IMP_OSD_SetRgnAttr, IMP_OSD_ShowRgn,
        IMP_OSD_Start, IMP_OSD_UnRegisterRgn, IMP_System_Exit, IMP_System_GetTimeStamp,
        IMP_System_Init, INVHANDLE,
    },
    imp_log_dbg, imp_log_err, imp_log_info, logodata_100x100_bgra, ret_verify,
};

const SENSOR_FRAME_RATE_NUM: i32 = 25;
const SENSOR_FRAME_RATE_DEN: i32 = 1;

use alloc::{slice, string::ToString};
use core::{
    borrow::Borrow,
    error,
    ffi::{c_void, CStr},
    fmt::{self, Binary, Write as bWrite},
    mem::{self, MaybeUninit},
    ptr,
    result::Result,
    time::{self, Duration},
};

use nix::sys::{
    select::{select, FdSet},
    time::TimeVal,
};
use nix::unistd::*;
use std::os::unix::io::{AsRawFd, BorrowedFd, FromRawFd, OwnedFd, RawFd};
use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
    os::{fd, unix::fs::OpenOptionsExt},
    thread,
};

#[derive(Debug, Default)]
struct SensorConfig {
    pub name: &'static str,
    pub cubs_type: IMPSensorControlBusType,
    pub i2c_addr: u8,
    pub i2c_adapter_id: u8,
    pub width: i32,
    pub height: i32,
    pub chn_enabled: [bool; 4], //chn0,chn1,chn2,chn3
    pub crop_enabled: usize,
    pub common: SensorCommon,
}
impl Default for IMPSensorControlBusType {
    fn default() -> Self {
        IMPSensorControlBusType::TX_SENSOR_CONTROL_INTERFACE_I2C
    }
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
    cubs_type: IMPSensorControlBusType::TX_SENSOR_CONTROL_INTERFACE_I2C,
    i2c_addr: 0x37,
    i2c_adapter_id: 0,
    width: 1920,
    height: 1080,
    chn_enabled: [true, false, false, false],
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

const NR_FRAMES_TO_SAVE: i32 = 200;
const NR_JPEG_TO_SAVE: u32 = 20;
const STREAM_BUFFER_SIZE: u32 = 1 * 1024 * 1024;

const ENC_VIDEO_CHANNEL: u32 = 0;
const ENC_JPEG_CHANNEL: u32 = 1;

const STREAM_FILE_PATH_PREFIX: &str = "/tmp\0";
const SNAP_FILE_PATH_PREFIX: &str = "/tmp\0";

pub const OSD_REGION_WIDTH: u32 = 16;
pub const OSD_REGION_HEIGHT: u32 = 34;
pub const OSD_REGION_WIDTH_SEC: u32 = 8;
pub const OSD_REGION_HEIGHT_SEC: u32 = 18;

const SLEEP_TIME: u64 = 1; // 秒

pub const FS_CHN_NUM: usize = 4; // MIN 1, MAX 4
const IVS_CHN_ID: usize = 2;

const CH0_INDEX: i32 = 0;
const CH1_INDEX: i32 = 1;
const CH2_INDEX: i32 = 2;
const CH3_INDEX: i32 = 3; // ext CHN

const CHN_ENABLE: usize = 1;
const CHN_DISABLE: usize = 0;

impl Default for IMPPayloadType {
    fn default() -> Self {
        IMPPayloadType::PT_JPEG
    }
}
#[derive(Debug, Default)]
pub struct ChnConf {
    pub index: i32,   // 0 表示主通道, 1 表示次通道
    pub enable: bool, // 通道是否启用
    pub payload_type: IMPPayloadType,
    pub fs_chn_attr: IMPFSChnAttr,
    pub framesource_chn: IMPCell,
    pub imp_encoder: IMPCell,
}
static CHN_NUM: usize = unsafe { CHN.len() };
const TAG: &str = "Sample";
const S_RC_METHOD: i32 = ENC_RC_MODE_CBR as i32;
static DIRECT_SWICTH: i32 = 0;
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
            deviceID: IMPDeviceID::DEV_ID_FS,
            groupID: CH2_INDEX,
            outputID: 0,
        },
        imp_encoder: IMPCell {
            deviceID: IMPDeviceID::DEV_ID_ENC,
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
            pixFmt: IMPPixelFormat::PIX_FMT_YUV420P,
            crop: IMPFSChnCrop::new(),
            scaler: IMPFSChnScaler::new(),
            outFrmRateDen: 0,
            outFrmRateNum: 0,
            nrVBs: 0,
            type_: IMPFSChnType::FS_PHY_CHANNEL,
            mirr_enable: 0,
            fcrop: IMPFSChnCrop::new(),
        }
    }
}

impl IMPCell {
    const fn new() -> Self {
        IMPCell {
            deviceID: IMPDeviceID::DEV_ID_FS,
            groupID: 0,
            outputID: 0,
        }
    }
}

impl ChnConf {
    const fn new() -> Self {
        ChnConf {
            index: 0,
            enable: false,
            payload_type: IMPPayloadType::PT_JPEG,
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

struct ByteArrayWriter<'a> {
    buffer: &'a mut [u8],
    position: usize,
}

impl<'a> bWrite for ByteArrayWriter<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let bytes = s.as_bytes();
        if self.position + bytes.len() > self.buffer.len() {
            return Err(fmt::Error);
        }
        self.buffer[self.position..self.position + bytes.len()].copy_from_slice(bytes);
        self.position += bytes.len();
        Ok(())
    }
}

extern "C" {
    pub fn IMP_Encoder_SetPoolSize(size: libc::c_int) -> libc::c_int;
    pub fn IMP_OSD_SetPoolSize(size: libc::c_int) -> libc::c_int;
}
use core::{borrow::BorrowMut, ptr::copy_nonoverlapping};

pub fn sample_system_init(sensor_info: *mut IMPSensorInfo) -> Result<i32, i32> {
    let mut ret: i32 = 0;
    let sensor_info = unsafe { &mut *sensor_info };
    unsafe {
        /* isp osd and ipu osd buffer size set */
        match GOSD_ENABLE {
            1 => {
                /* only use ipu osd */
                IMP_OSD_SetPoolSize(512 * 1024);
            }
            2 => {
                /* only use isp osd */
                IMP_ISP_Tuning_SetOsdPoolSize(512 * 1024);
            }
            3 => {
                /* use ipu osd and isp osd */
                IMP_OSD_SetPoolSize(512 * 1024);
                IMP_ISP_Tuning_SetOsdPoolSize(512 * 1024);
            }
            _ => {
                IMP_OSD_SetPoolSize(512 * 1024);
                IMP_ISP_Tuning_SetOsdPoolSize(512 * 1024);
            }
        }
    }
    unsafe {
        copy_nonoverlapping(
            SENSOR_CONFIG.name.as_ptr(),
            sensor_info.name.as_mut_ptr() as *mut u8,
            SENSOR_CONFIG.name.len(),
        );
        sensor_info.cbus_type = SENSOR_CONFIG.cubs_type;

        copy_nonoverlapping(
            SENSOR_CONFIG.name.as_ptr(),
            sensor_info.__anon1.i2c.type_.as_mut_ptr() as *mut u8,
            SENSOR_CONFIG.name.len(),
        );
        sensor_info.__anon1.i2c.addr = SENSOR_CONFIG.i2c_addr as _;
        sensor_info.__anon1.i2c.i2c_adapter_id = SENSOR_CONFIG.i2c_adapter_id as _;

        imp_log_info!(&TAG, "sample_system_init() start ");

        ret = IMP_ISP_Open();
        ret_verify!(ret, "failed to open ISP\n");

        ret = IMP_ISP_AddSensor(sensor_info);
        ret_verify!(ret, "failed to AddSensor\n");

        ret = IMP_ISP_EnableSensor();
        ret_verify!(ret, "failed to EnableSensor\n");

        ret = IMP_System_Init();
        ret_verify!(ret, "IMP_System_Init failed\n");

        /* enable turning, to debug graphics */
        ret = IMP_ISP_EnableTuning();
        ret_verify!(ret, "IMP_ISP_EnableTuning failed\n");

        IMP_ISP_Tuning_SetContrast(128);
        IMP_ISP_Tuning_SetSharpness(128);
        IMP_ISP_Tuning_SetSaturation(128);
        IMP_ISP_Tuning_SetBrightness(128);

        ret = IMP_ISP_Tuning_SetISPRunningMode(IMPISP_RUNNING_MODE_DAY);
        ret_verify!(ret, "failed to set running mode\n");

        imp_log_info!(TAG, "ImpSystemInit success");
    }

    Ok(0)
}

pub fn sample_system_exit(sensor_info: *mut IMPSensorInfo) -> Result<(), i32> {
    let mut ret: i32 = 0;
    unsafe {
        imp_log_info!(&TAG, "sample_system_exit start");
        IMP_System_Exit();

        ret = IMP_ISP_DisableSensor();
        ret_verify!(ret, "failed to DisableSensor\n");

        ret = IMP_ISP_DelSensor(sensor_info);
        ret_verify!(ret, "failed to DelSensor\n");

        ret = IMP_ISP_DisableTuning();
        ret_verify!(ret, "IMP_ISP_DisableTuning failed\n");

        if IMP_ISP_Close() != 0 {
            imp_log_err!(&TAG, "failed to open ISP");
            return Err(-1);
        }
        imp_log_info!(&TAG, "sample_system_exit success !");
    }
    Ok(())
}

pub fn sample_framesource_streamon() -> Result<(), i32> {
    let mut ret: i32;

    for i in 0..FS_CHN_NUM {
        unsafe {
            if CHN[i].enable {
                ret = IMP_FrameSource_EnableChn(CHN[i].index);
                ret_verify!(ret, "IMP_FrameSource_EnableChn() error !\n\0",);
            }
        }
    }
    Ok(())
}

pub fn sample_framesource_streamoff() -> Result<(), i32> {
    let mut ret: i32;

    for i in 0..FS_CHN_NUM {
        unsafe {
            if CHN[i].enable {
                ret = IMP_FrameSource_DisableChn(CHN[i].index);
                ret_verify!(ret, "IMP_FrameSource_DisableChn() error !\n\0",);
            }
        }
    }
    Ok(())
}

pub fn sample_framesource_init() -> Result<i32, i32> {
    let mut ret = 0;
    let mut i: usize;

    for i in 0..FS_CHN_NUM {
        unsafe {
            if CHN[i].enable {
                ret = IMP_FrameSource_CreateChn(CHN[i].index, &mut CHN[i].fs_chn_attr);
                ret_verify!(ret, "IMP_FrameSource_CreateChn() err - common:632\n");

                ret = IMP_FrameSource_SetChnAttr(CHN[i].index, &mut CHN[i].fs_chn_attr);
                ret_verify!(ret, "IMP_FrameSource_SetChnAttr() err - common:635\n");
            }
        }
    }
    Ok(0)
}

pub fn sample_framesource_exit() -> Result<(), i32> {
    let mut ret = 0;
    let mut i: usize;

    for i in 0..FS_CHN_NUM {
        unsafe {
            if CHN[i].enable {
                ret = IMP_FrameSource_DestroyChn(CHN[i].index);
                ret_verify!(ret, "IMP_FrameSource_CreateChn(CHN{}) err\n", CHN[i].index);
            }
        }
    }
    Ok(())
}

pub fn sample_encoder_init() -> Result<(), i32> {
    let mut i: usize;
    let mut ret = 0;
    let rc_attr: *mut IMPEncoderAttr;

    for i in 0..FS_CHN_NUM {
        unsafe {
            if CHN[i].enable {
                let imp_chn_attr_tmp: &IMPFSChnAttr = &CHN[i].fs_chn_attr;
                let mut channel_attr = IMPEncoderCHNAttr::default();
                let enc_attr = &mut channel_attr.encAttr;
                enc_attr.enType = CHN[i].payload_type;

                // 根据图像高度设置缓冲区大小
                enc_attr.bufSize = if imp_chn_attr_tmp.picHeight >= 1920 {
                    (imp_chn_attr_tmp.picWidth * imp_chn_attr_tmp.picHeight * 3 / 10) as _
                } else if imp_chn_attr_tmp.picHeight >= 1520 {
                    (imp_chn_attr_tmp.picWidth * imp_chn_attr_tmp.picHeight * 3 / 8) as _
                } else if imp_chn_attr_tmp.picHeight >= 1080 {
                    (imp_chn_attr_tmp.picWidth * imp_chn_attr_tmp.picHeight / 2) as _
                } else {
                    (imp_chn_attr_tmp.picWidth * imp_chn_attr_tmp.picHeight * 3 / 4) as _
                };

                enc_attr.profile = 1;
                enc_attr.picWidth = imp_chn_attr_tmp.picWidth as _;
                enc_attr.picHeight = imp_chn_attr_tmp.picHeight as _;

                let mut chn_num;
                if CHN[i].payload_type == PT_JPEG {
                    chn_num = 3 + CHN[i].index;
                } else if CHN[i].payload_type == PT_H264 {
                    chn_num = CHN[i].index;
                    let rc_attr = &mut channel_attr.rcAttr;
                    rc_attr.outFrmRate.frmRateNum = imp_chn_attr_tmp.outFrmRateNum as _;
                    rc_attr.outFrmRate.frmRateDen = imp_chn_attr_tmp.outFrmRateDen as _;
                    rc_attr.maxGop =
                        2 * rc_attr.outFrmRate.frmRateNum / rc_attr.outFrmRate.frmRateDen;

                    if S_RC_METHOD == ENC_RC_MODE_CBR as _ {
                        rc_attr.attrRcMode.rcMode = ENC_RC_MODE_CBR;
                        rc_attr.attrRcMode.__anon1.attrH264Cbr.outBitRate = BITRATE_720P_Kbs;
                        rc_attr.attrRcMode.__anon1.attrH264Cbr.maxQp = 45;
                        rc_attr.attrRcMode.__anon1.attrH264Cbr.minQp = 15;
                        rc_attr.attrRcMode.__anon1.attrH264Cbr.iBiasLvl = 0;
                        rc_attr.attrRcMode.__anon1.attrH264Cbr.frmQPStep = 3;
                        rc_attr.attrRcMode.__anon1.attrH264Cbr.gopQPStep = 15;
                        rc_attr.attrRcMode.__anon1.attrH264Cbr.adaptiveMode = false;
                        rc_attr.attrRcMode.__anon1.attrH264Cbr.gopRelation = false;

                        rc_attr.attrHSkip.hSkipAttr.skipType = IMP_Encoder_STYPE_N1X;
                        rc_attr.attrHSkip.hSkipAttr.m = 0;
                        rc_attr.attrHSkip.hSkipAttr.n = 0;
                        rc_attr.attrHSkip.hSkipAttr.maxSameSceneCnt = 0;
                        rc_attr.attrHSkip.hSkipAttr.bEnableScenecut = false as _;
                        rc_attr.attrHSkip.hSkipAttr.bBlackEnhance = false as _;
                        rc_attr.attrHSkip.maxHSkipType = IMP_Encoder_STYPE_N1X;
                    } else if S_RC_METHOD == ENC_RC_MODE_VBR as _ {
                        rc_attr.attrRcMode.rcMode = ENC_RC_MODE_VBR;
                        rc_attr.attrRcMode.__anon1.attrH264Vbr.maxQp = 45;
                        rc_attr.attrRcMode.__anon1.attrH264Vbr.minQp = 15;
                        rc_attr.attrRcMode.__anon1.attrH264Vbr.staticTime = 2;
                        rc_attr.attrRcMode.__anon1.attrH264Vbr.maxBitRate = BITRATE_720P_Kbs;
                        rc_attr.attrRcMode.__anon1.attrH264Vbr.iBiasLvl = 0;
                        rc_attr.attrRcMode.__anon1.attrH264Vbr.changePos = 80;
                        rc_attr.attrRcMode.__anon1.attrH264Vbr.qualityLvl = 2;
                        rc_attr.attrRcMode.__anon1.attrH264Vbr.frmQPStep = 3;
                        rc_attr.attrRcMode.__anon1.attrH264Vbr.gopQPStep = 15;
                        rc_attr.attrRcMode.__anon1.attrH264Vbr.gopRelation = false;

                        rc_attr.attrHSkip.hSkipAttr.skipType = IMP_Encoder_STYPE_N1X;
                        rc_attr.attrHSkip.hSkipAttr.m = 0;
                        rc_attr.attrHSkip.hSkipAttr.n = 0;
                        rc_attr.attrHSkip.hSkipAttr.maxSameSceneCnt = 0;
                        rc_attr.attrHSkip.hSkipAttr.bEnableScenecut = false as _;
                        rc_attr.attrHSkip.hSkipAttr.bBlackEnhance = false as _;
                        rc_attr.attrHSkip.maxHSkipType = IMP_Encoder_STYPE_N1X;
                    } else if S_RC_METHOD == ENC_RC_MODE_SMART as _ {
                        rc_attr.attrRcMode.rcMode = ENC_RC_MODE_SMART;
                        rc_attr.attrRcMode.__anon1.attrH264Smart.maxQp = 45;
                        rc_attr.attrRcMode.__anon1.attrH264Smart.minQp = 15;
                        rc_attr.attrRcMode.__anon1.attrH264Smart.staticTime = 2;
                        rc_attr.attrRcMode.__anon1.attrH264Smart.maxBitRate = BITRATE_720P_Kbs;
                        rc_attr.attrRcMode.__anon1.attrH264Smart.iBiasLvl = 0;
                        rc_attr.attrRcMode.__anon1.attrH264Smart.changePos = 80;
                        rc_attr.attrRcMode.__anon1.attrH264Smart.qualityLvl = 2;
                        rc_attr.attrRcMode.__anon1.attrH264Smart.frmQPStep = 3;
                        rc_attr.attrRcMode.__anon1.attrH264Smart.gopQPStep = 15;
                        rc_attr.attrRcMode.__anon1.attrH264Smart.gopRelation = false;

                        rc_attr.attrHSkip.hSkipAttr.skipType = IMP_Encoder_STYPE_N1X;
                        rc_attr.attrHSkip.hSkipAttr.m = (rc_attr.maxGop - 1) as _;
                        rc_attr.attrHSkip.hSkipAttr.n = 1;
                        rc_attr.attrHSkip.hSkipAttr.maxSameSceneCnt = 6;
                        rc_attr.attrHSkip.hSkipAttr.bEnableScenecut = false as _;
                        rc_attr.attrHSkip.hSkipAttr.bBlackEnhance = false as _;
                        rc_attr.attrHSkip.maxHSkipType = IMP_Encoder_STYPE_N1X;
                    } else {
                        /* fixQp */
                        rc_attr.attrRcMode.rcMode = ENC_RC_MODE_FIXQP;
                        rc_attr.attrRcMode.__anon1.attrH264FixQp.qp = 35;

                        rc_attr.attrHSkip.hSkipAttr.skipType = IMP_Encoder_STYPE_N1X;
                        rc_attr.attrHSkip.hSkipAttr.m = 0;
                        rc_attr.attrHSkip.hSkipAttr.n = 0;
                        rc_attr.attrHSkip.hSkipAttr.maxSameSceneCnt = 0;
                        rc_attr.attrHSkip.hSkipAttr.bEnableScenecut = false as _;
                        rc_attr.attrHSkip.hSkipAttr.bBlackEnhance = false as _;
                        rc_attr.attrHSkip.maxHSkipType = IMP_Encoder_STYPE_N1X;
                    }
                } else {
                    // PT_H265
                    chn_num = CHN[i].index;
                    let rc_attr = &mut channel_attr.rcAttr;
                    rc_attr.outFrmRate.frmRateNum = imp_chn_attr_tmp.outFrmRateNum as _;
                    rc_attr.outFrmRate.frmRateDen = imp_chn_attr_tmp.outFrmRateDen as _;
                    rc_attr.maxGop =
                        2 * rc_attr.outFrmRate.frmRateNum / rc_attr.outFrmRate.frmRateDen;

                    if S_RC_METHOD == ENC_RC_MODE_CBR as _ {
                        rc_attr.attrRcMode.__anon1.attrH265Cbr.outBitRate = BITRATE_720P_Kbs;
                        rc_attr.attrRcMode.__anon1.attrH265Cbr.maxQp = 45;
                        rc_attr.attrRcMode.__anon1.attrH265Cbr.minQp = 15;
                        rc_attr.attrRcMode.__anon1.attrH265Cbr.staticTime = 2;

                        rc_attr.attrRcMode.__anon1.attrH265Cbr.iBiasLvl = 0;
                        rc_attr.attrRcMode.__anon1.attrH265Cbr.frmQPStep = 3;
                        rc_attr.attrRcMode.__anon1.attrH265Cbr.gopQPStep = 15;
                        rc_attr.attrRcMode.__anon1.attrH265Cbr.flucLvl = 2;

                        rc_attr.attrHSkip.hSkipAttr.skipType = IMP_Encoder_STYPE_N1X;
                        rc_attr.attrHSkip.hSkipAttr.m = 0;
                        rc_attr.attrHSkip.hSkipAttr.n = 0;
                        rc_attr.attrHSkip.hSkipAttr.maxSameSceneCnt = 0;
                        rc_attr.attrHSkip.hSkipAttr.bEnableScenecut = false as _;
                        rc_attr.attrHSkip.hSkipAttr.bBlackEnhance = false as _;
                        rc_attr.attrHSkip.maxHSkipType = IMP_Encoder_STYPE_N1X;
                    } else if S_RC_METHOD == ENC_RC_MODE_VBR as _ {
                        rc_attr.attrRcMode.rcMode = ENC_RC_MODE_VBR;
                        rc_attr.attrRcMode.__anon1.attrH265Vbr.maxQp = 45;
                        rc_attr.attrRcMode.__anon1.attrH265Vbr.minQp = 15;
                        rc_attr.attrRcMode.__anon1.attrH265Vbr.staticTime = 2;
                        rc_attr.attrRcMode.__anon1.attrH265Vbr.maxBitRate = BITRATE_720P_Kbs;
                        rc_attr.attrRcMode.__anon1.attrH265Vbr.iBiasLvl = 0;
                        rc_attr.attrRcMode.__anon1.attrH265Vbr.changePos = 80;
                        rc_attr.attrRcMode.__anon1.attrH265Vbr.qualityLvl = 2;
                        rc_attr.attrRcMode.__anon1.attrH265Vbr.frmQPStep = 3;
                        rc_attr.attrRcMode.__anon1.attrH265Vbr.gopQPStep = 15;
                        rc_attr.attrRcMode.__anon1.attrH265Vbr.flucLvl = 2;

                        rc_attr.attrHSkip.hSkipAttr.skipType = IMP_Encoder_STYPE_N1X;
                        rc_attr.attrHSkip.hSkipAttr.m = 0;
                        rc_attr.attrHSkip.hSkipAttr.n = 0;
                        rc_attr.attrHSkip.hSkipAttr.maxSameSceneCnt = 0;
                        rc_attr.attrHSkip.hSkipAttr.bEnableScenecut = false as _;
                        rc_attr.attrHSkip.hSkipAttr.bBlackEnhance = false as _;
                        rc_attr.attrHSkip.maxHSkipType = IMP_Encoder_STYPE_N1X;
                    } else if S_RC_METHOD == ENC_RC_MODE_SMART as _ {
                        rc_attr.attrRcMode.rcMode = ENC_RC_MODE_SMART;
                        rc_attr.attrRcMode.__anon1.attrH265Smart.maxQp = 45;
                        rc_attr.attrRcMode.__anon1.attrH265Smart.minQp = 15;
                        rc_attr.attrRcMode.__anon1.attrH265Smart.staticTime = 2;
                        rc_attr.attrRcMode.__anon1.attrH265Smart.maxBitRate = BITRATE_720P_Kbs;
                        rc_attr.attrRcMode.__anon1.attrH265Smart.iBiasLvl = 0;
                        rc_attr.attrRcMode.__anon1.attrH265Smart.changePos = 80;
                        rc_attr.attrRcMode.__anon1.attrH265Smart.qualityLvl = 2;
                        rc_attr.attrRcMode.__anon1.attrH265Smart.frmQPStep = 3;
                        rc_attr.attrRcMode.__anon1.attrH265Smart.gopQPStep = 15;
                        rc_attr.attrRcMode.__anon1.attrH265Smart.flucLvl = 2;

                        rc_attr.attrHSkip.hSkipAttr.skipType = IMP_Encoder_STYPE_N1X;
                        rc_attr.attrHSkip.hSkipAttr.m = (rc_attr.maxGop - 1) as _;
                        rc_attr.attrHSkip.hSkipAttr.n = 1;
                        rc_attr.attrHSkip.hSkipAttr.maxSameSceneCnt = 6;
                        rc_attr.attrHSkip.hSkipAttr.bEnableScenecut = false as _;
                        rc_attr.attrHSkip.hSkipAttr.bBlackEnhance = false as _;
                        rc_attr.attrHSkip.maxHSkipType = IMP_Encoder_STYPE_N1X;
                    } else {
                        /* fixQp */
                        rc_attr.attrRcMode.rcMode = ENC_RC_MODE_FIXQP;
                        rc_attr.attrRcMode.__anon1.attrH265FixQp.qp = 35;

                        rc_attr.attrHSkip.hSkipAttr.skipType = IMP_Encoder_STYPE_N1X;
                        rc_attr.attrHSkip.hSkipAttr.m = 0;
                        rc_attr.attrHSkip.hSkipAttr.n = 0;
                        rc_attr.attrHSkip.hSkipAttr.maxSameSceneCnt = 0;
                        rc_attr.attrHSkip.hSkipAttr.bEnableScenecut = false as _;
                        rc_attr.attrHSkip.hSkipAttr.bBlackEnhance = false as _;
                        rc_attr.attrHSkip.maxHSkipType = IMP_Encoder_STYPE_N1X;
                    }
                }
                if DIRECT_SWICTH == 1 && chn_num == 0 {
                    channel_attr.bEnableIvdc = true;
                }
                imp_log_info!(
                    &TAG,
                    "IMP_Encoder_CreateChn[{}] CHN.enable = {}",
                    i,
                    CHN[i].enable
                );

                let ret = IMP_Encoder_CreateChn(chn_num, &channel_attr);
                ret_verify!(ret, "IMP_Encoder_CreateChn() error!");

                let ret = IMP_Encoder_RegisterChn(CHN[i].index, chn_num);
                ret_verify!(ret, "IMP_Encoder_RegisterChn fail!");
            }
        }
    }
    Ok(())
}

pub fn sample_encoder_exit() -> Result<(), i32> {
    let mut ret: i32;
    let mut chn_num: i32;
    let mut chn_stat: IMPEncoderCHNStat = Default::default();

    for i in 0..FS_CHN_NUM {
        if unsafe { CHN[i].enable } {
            chn_num = if unsafe { CHN[i].payload_type } == PT_JPEG {
                3 + unsafe { CHN[i].index }
            } else {
                unsafe { CHN[i].index }
            };
            ret = unsafe { IMP_Encoder_Query(chn_num, &mut chn_stat) };
            ret_verify!(ret, "IMP_Encoder_Query error\n");

            if chn_stat.registered {
                ret = unsafe { IMP_Encoder_UnRegisterChn(chn_num) };
                ret_verify!(ret, "IMP_Encoder_UnRegisterChn() error: \n");

                ret = unsafe { IMP_Encoder_DestroyChn(chn_num) };
                ret_verify!(ret, "IMP_Encoder_DestroyChn({}) error:\n", chn_num);
            }
        }
    }
    Ok(())
}

pub fn sample_osd_init(grp_num: i32) -> Result<Vec<IMPRgnHandle>, i32> {
    let mut ret: i32;
    let mut handles: Vec<IMPRgnHandle> = Vec::with_capacity(4);

    let mut r_hander_font = unsafe { IMP_OSD_CreateRgn(ptr::null_mut()) };
    ret_verify!(r_hander_font, "IMP_OSD_CreateRgn TimeStamp error!");
    handles.push(r_hander_font);

    let mut r_hander_logo = unsafe { IMP_OSD_CreateRgn(ptr::null_mut()) };
    ret_verify!(r_hander_font, "IMP_OSD_CreateRgn Logo error!");
    handles.push(r_hander_logo);

    let mut r_hander_cover = unsafe { IMP_OSD_CreateRgn(ptr::null_mut()) };
    ret_verify!(r_hander_font, "IMP_OSD_CreateRgn Cover error!");
    handles.push(r_hander_cover);

    let mut r_hander_rect = unsafe { IMP_OSD_CreateRgn(ptr::null_mut()) };
    ret_verify!(r_hander_font, "IMP_OSD_CreateRgn Rect error!");
    handles.push(r_hander_rect);

    for &r_hander in &handles {
        let ret = unsafe { IMP_OSD_RegisterRgn(r_hander, grp_num, ptr::null_mut()) };
        ret_verify!(ret, "IVS IMP_OSD_RegisterRgn failed");
    }

    // 设置 Region 属性
    let mut r_attr_font = IMPOSDRgnAttr::default();
    r_attr_font.type_ = OSD_REG_PIC;
    r_attr_font.rect.p0.x = 10;
    r_attr_font.rect.p0.y = 10;
    r_attr_font.rect.p1.x = r_attr_font.rect.p0.x + 20 * (OSD_REGION_WIDTH as i32) - 1; //p0 is start，and p1 well be epual p0+width(or heigth)-1
    r_attr_font.rect.p1.y = r_attr_font.rect.p0.y + OSD_REGION_HEIGHT as i32 - 1;

    r_attr_font.fmt = {
        #[cfg(feature = "SUPPORT_RGB555LE")]
        {
            PIX_FMT_RGB555LE
        }
        #[cfg(not(feature = "SUPPORT_RGB555LE"))]
        {
            PIX_FMT_BGRA
        }
    };

    r_attr_font.data.picData.pData = ptr::null::<c_void>() as *mut c_void;

    ret = unsafe { IMP_OSD_SetRgnAttr(r_hander_font, &mut r_attr_font) };
    ret_verify!(ret, "IVS IMP_OSD_SetRgnAttr failed");

    // 获取和设置 Group Region 属性
    let mut gr_attr_font = IMPOSDGrpRgnAttr::default();
    ret = unsafe { IMP_OSD_GetGrpRgnAttr(r_hander_font, grp_num, &mut gr_attr_font) };
    ret_verify!(ret, "IVS IMP_OSD_SetRgnAttr failed");

    gr_attr_font.show = 0;

    /* Disable Font global alpha, only use pixel alpha. */
    gr_attr_font.gAlphaEn = 1;
    gr_attr_font.fgAlhpa = 0xff;
    gr_attr_font.layer = 3;

    ret = unsafe { IMP_OSD_SetGrpRgnAttr(r_hander_font, grp_num, &mut gr_attr_font) };
    ret_verify!(ret, "IMP_OSD_SetGrpRgnAttr Font error");

    let mut r_attr_logo = IMPOSDRgnAttr::default();
    let picw = 100;
    let pich = 100;
    r_attr_logo.type_ = OSD_REG_PIC;
    r_attr_logo.rect.p0.x = SENSOR_CONFIG.width - 100;
    r_attr_logo.rect.p0.y = SENSOR_CONFIG.height - 100;
    r_attr_logo.rect.p1.x = r_attr_logo.rect.p0.x + picw - 1; //p0 is start，and p1 well be epual p0+width(or heigth)-1
    r_attr_logo.rect.p1.y = r_attr_logo.rect.p0.y + pich - 1;
    r_attr_logo.fmt = PIX_FMT_BGRA;
    r_attr_logo.data.picData.pData =
        logodata_100x100_bgra::logodata_100x100_bgra.as_mut_ptr() as *mut c_void;

    ret = unsafe { IMP_OSD_SetRgnAttr(r_hander_logo, &mut r_attr_logo) };
    ret_verify!(ret, "IMP_OSD_SetRgnAttr Logo error");

    let mut gr_attr_logo = IMPOSDGrpRgnAttr::default();

    ret = unsafe { IMP_OSD_GetGrpRgnAttr(r_hander_logo, grp_num, &mut gr_attr_logo) };
    ret_verify!(ret, "IMP_OSD_GetGrpRgnAttr Logo error");

    gr_attr_logo.show = 0;
    gr_attr_logo.gAlphaEn = 1;

    /* Set Logo global alpha to 0x7f, it is semi-transparent. */
    gr_attr_logo.fgAlhpa = 0x7f;
    gr_attr_logo.layer = 2;

    ret = unsafe { IMP_OSD_SetGrpRgnAttr(r_hander_logo, grp_num, &mut gr_attr_logo) };
    ret_verify!(ret, "IMP_OSD_SetGrpRgnAttr Logo error");

    // 设置Cover区域
    let mut r_attr_cover = IMPOSDRgnAttr::default();
    r_attr_cover.type_ = OSD_REG_COVER;
    r_attr_cover.rect.p0.x = SENSOR_CONFIG.width / 2 - 100;
    r_attr_cover.rect.p0.y = SENSOR_CONFIG.height / 2 - 100;
    r_attr_cover.rect.p1.x = r_attr_cover.rect.p0.x + SENSOR_CONFIG.width / 2 - 1 + 50;
    r_attr_cover.rect.p1.y = r_attr_cover.rect.p0.y + SENSOR_CONFIG.height / 2 - 1 + 50;
    r_attr_cover.fmt = PIX_FMT_BGRA;
    r_attr_cover.data.coverData.color = OSD_BLACK as _;

    ret = unsafe { IMP_OSD_SetRgnAttr(r_hander_cover, &mut r_attr_cover) };
    ret_verify!(ret, "IMP_OSD_SetRgnAttr Cover error");

    let mut gr_attr_cover = IMPOSDGrpRgnAttr::default();
    ret = unsafe { IMP_OSD_GetGrpRgnAttr(r_hander_cover, grp_num, &mut gr_attr_cover) };
    ret_verify!(ret, "IMP_OSD_GetGrpRgnAttr Cover error");

    gr_attr_cover.show = 0;

    /* Disable Cover global alpha, it is absolutely no transparent. */
    gr_attr_cover.gAlphaEn = 1;
    gr_attr_cover.fgAlhpa = 0x7f; // 完全不透明
    gr_attr_cover.layer = 2;

    ret = unsafe { IMP_OSD_SetGrpRgnAttr(r_hander_cover, grp_num, &mut gr_attr_cover) };
    ret_verify!(ret, "IMP_OSD_SetGrpRgnAttr Cover error");

    let mut r_attr_rect = IMPOSDRgnAttr::default();
    r_attr_rect.type_ = OSD_REG_RECT;
    r_attr_rect.rect.p0.x = 300;
    r_attr_rect.rect.p0.y = 300;
    r_attr_rect.rect.p1.x = r_attr_rect.rect.p0.x + 600 - 1;
    r_attr_rect.rect.p1.y = r_attr_rect.rect.p0.y + 300 - 1;
    r_attr_rect.fmt = PIX_FMT_MONOWHITE;
    r_attr_rect.data.lineRectData.color = OSD_GREEN as _;
    r_attr_rect.data.lineRectData.linewidth = 5;

    ret = unsafe { IMP_OSD_SetRgnAttr(r_hander_rect, &mut r_attr_rect) };
    ret_verify!(ret, "IMP_OSD_SetRgnAttr Rect error");

    let mut gr_attr_rect = IMPOSDGrpRgnAttr::default();
    ret = unsafe { IMP_OSD_GetGrpRgnAttr(r_hander_rect, grp_num, &mut gr_attr_rect) };
    ret_verify!(ret, "IMP_OSD_GetGrpRgnAttr Rect error");

    gr_attr_rect.show = 0;
    gr_attr_rect.layer = 1;
    gr_attr_rect.scalex = 1.0;
    gr_attr_rect.scaley = 1.0;

    ret = unsafe { IMP_OSD_SetGrpRgnAttr(r_hander_rect, grp_num, &mut gr_attr_rect) };
    ret_verify!(ret, "IMP_OSD_SetGrpRgnAttr Rect error");

    ret = unsafe { IMP_OSD_Start(grp_num) };
    ret_verify!(ret, "IMP_OSD_Start failed");

    imp_log_info!(&TAG, "sample osd init success !");
    // let raw_ptr = handles.as_mut_ptr();
    // mem::forget(handles);
    Ok(handles)
}

pub fn sample_osd_exit(pr_handers: Vec<IMPRgnHandle>, grp_num: i32) -> Result<(), i32> {
    let mut ret: i32;
    unsafe {
        ret = IMP_OSD_ShowRgn(pr_handers[0], grp_num, 0);
        if ret < 0 {
            imp_log_err!(&TAG, "IMP_OSD_ShowRgn close timeStamp error\n");
        }

        ret = IMP_OSD_ShowRgn(pr_handers[1], grp_num, 0);
        if ret < 0 {
            imp_log_err!(&TAG, "IMP_OSD_ShowRgn close Logo error\n");
        }

        ret = IMP_OSD_ShowRgn(pr_handers[2], grp_num, 0);
        if ret < 0 {
            imp_log_err!(&TAG, "IMP_OSD_ShowRgn close cover error\n");
        }

        ret = IMP_OSD_ShowRgn(pr_handers[3], grp_num, 0);
        if ret < 0 {
            imp_log_err!(&TAG, "IMP_OSD_ShowRgn close Rect error\n");
        }

        // 注销各个 Region
        ret = IMP_OSD_UnRegisterRgn(pr_handers[0], grp_num);
        if ret < 0 {
            imp_log_err!(&TAG, "IMP_OSD_UnRegisterRgn timeStamp error\n");
        }

        ret = IMP_OSD_UnRegisterRgn(pr_handers[1], grp_num);
        if ret < 0 {
            imp_log_err!(&TAG, "IMP_OSD_UnRegisterRgn logo error\n");
        }

        ret = IMP_OSD_UnRegisterRgn(pr_handers[2], grp_num);
        if ret < 0 {
            imp_log_err!(&TAG, "IMP_OSD_UnRegisterRgn Cover error\n");
        }

        ret = IMP_OSD_UnRegisterRgn(pr_handers[3], grp_num);
        if ret < 0 {
            imp_log_err!(&TAG, "IMP_OSD_UnRegisterRgn Rect error\n");
        }

        IMP_OSD_DestroyRgn(pr_handers[0]);
        IMP_OSD_DestroyRgn(pr_handers[1]);
        IMP_OSD_DestroyRgn(pr_handers[2]);
        IMP_OSD_DestroyRgn(pr_handers[3]);

        ret = IMP_OSD_DestroyGroup(grp_num);
        ret_verify!(ret, "IMP_OSD_DestroyGroup(0) error\n");

        //nix::libc::free(pr_handers as *mut c_void);
    }

    Ok(())
}

pub fn save_stream(file: &mut File, stream: &IMPEncoderStream) -> io::Result<()> {
    let nr_pack = stream.packCount;

    for i in 0..nr_pack {
        let pack_ptr = unsafe { stream.pack.offset(i as isize) };
        let pack = unsafe { *pack_ptr };

        //let ret = unsafe { write(fd, pack.virAddr as *const _, pack.length as usize) };
        let data =
            unsafe { slice::from_raw_parts(pack.virAddr as *const u8, pack.length as usize) };

        match (*file).write_all(data) {
            Ok(_) => {}
            Err(e) => {
                imp_log_err!(&TAG, "stream write failed: {}", e);
                return Err(e);
            }
        }
    }
    Ok(())
}

pub fn save_stream_by_name(
    stream_prefix: &str,
    idx: i32,
    stream: &IMPEncoderStream,
) -> io::Result<()> {
    let stream_fd: i32;
    let mut stream_path = [0u8; 128];
    let mut ret: isize;

    let stream_path = format!("{}_{}", stream_prefix, idx);

    imp_log_info!(&TAG, "Openning Stream file");

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o777)
        .open(&stream_path)
        .map_err(|e| {
            imp_log_err!(&TAG, "Failed to open stream file: {}", e);
            e
        })?;

    //let stream = stream;
    let nr_pack = stream.packCount;

    for i in 0..nr_pack {
        let pack = unsafe { &*stream.pack.offset(i as isize) };

        let data =
            unsafe { slice::from_raw_parts(pack.virAddr as *const u8, pack.length as usize) };
        if let Err(e) = file.write_all(data) {
            imp_log_err!(&TAG, "Stream write error: {}", e);
            return Err(e);
        }
    }
    Ok(())
}
pub extern "C" fn libc_get_video_stream(val: *mut c_void) -> *mut c_void {
    let mut arg: i32 = unsafe { *(val as *mut i32) };
    get_video_stream(&mut arg).unwrap();
    return ptr::null_mut();
}
fn get_video_stream(arg: &mut i32) -> Result<(), i32> {
    let val = *arg;
    let chn_num = val & 0xffff;
    let payload_type = (val >> 16) & 0xffff;
    let stream_fd: i32;
    let total_save_cnt: i32;

    let mut ret = unsafe { IMP_Encoder_StartRecvPic(chn_num) };
    ret_verify!(ret, "IMP_Encoder_StartRecvPic(0) failed! - common:1218");

    let stream_suffix: &str = match payload_type {
        x if x == PT_H264 as i32 => "h264",
        x if x == PT_H265 as i32 => "h265",
        x if x == PT_JPEG as i32 => "jpeg",
        _ => "none\0",
    };

    let stream_path = format!(
        "{}/stream-{}.{}",
        STREAM_FILE_PATH_PREFIX, chn_num, stream_suffix
    );
    println!("file path = {}", stream_path);

    let (mut stream_fd, total_save_cnt) = if payload_type == PT_JPEG as i32 {
        let total_save_cnt = if NR_FRAMES_TO_SAVE / 50 > 0 {
            NR_FRAMES_TO_SAVE / 50
        } else {
            1
        };
        (None, total_save_cnt)
    } else {
        // 打开文件
        match OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0777)
            .open(&stream_path)
        {
            Ok(file) => {
                println!("open stream file ok");
                (Some(file), NR_FRAMES_TO_SAVE)
            }
            Err(e) => {
                eprintln!("Failed to open stream file: {}", e);
                return Err(-1);
            }
        }
    };

    for i in 0..total_save_cnt {
        ret = unsafe { IMP_Encoder_PollingStream(chn_num, 1000) };
        if ret < 0 {
            imp_log_err!(&TAG, "IMP_Encoder_PollingStream({}) timeout", chn_num);
            continue;
        }

        let mut stream: IMPEncoderStream = unsafe { mem::zeroed() };
        ret = unsafe { IMP_Encoder_GetStream(chn_num, &mut stream as *mut _, true) };

        ret_verify!(ret, "IMP_Encoder_GetStream({}) failed", chn_num);

        #[cfg(feature = "SHOW_FRM_BITRATE")]
        {
            let mut len = 0;

            for i in 0..stream.packCount {
                len += stream.pack[i].length as i32;
            }

            BITRATE_SP[chn_num] += len;
            FRMRATE_SP[chn_num] += 1;

            let now = unsafe { IMP_System_GetTimeStamp() } / 1000;

            if ((now - STATIME_SP[chn_num]) / 1000) as i32 >= FRM_BIT_RATE_TIME {
                let fps =
                    FRMRATE_SP[chn_num] as f64 / ((now - STATIME_SP[chn_num]) as f64 / 1000.0);
                let kbr = BITRATE_SP[chn_num] as f64 * 8.0 / (now - STATIME_SP[chn_num]) as f64;

                // println!("streamNum[{}]: FPS: {:.2}, Bitrate: {:.2} (kbps)", chn_num, fps, kbr);

                FRMRATE_SP[chn_num] = 0;
                BITRATE_SP[chn_num] = 0;
                STATIME_SP[chn_num] = now;
            }
        }

        if payload_type == PT_JPEG as i32 {
            if let Err(_) = save_stream_by_name(&stream_path, i, &stream) {
                return Err(-1);
            }
        } else if let Some(ref mut fd) = stream_fd {
            if let Err(e) = save_stream(fd, &stream) {
                imp_log_err!(&TAG, "{}", e);
                return Err(-1);
            }
        }

        unsafe { IMP_Encoder_ReleaseStream(chn_num, &mut stream) };
    }

    ret = unsafe { IMP_Encoder_StopRecvPic(chn_num) };

    ret_verify!(ret, "IMP_Encoder_StopRecvPic({}) failed", chn_num);

    Ok(())
}

pub fn sample_get_video_stream() -> Result<(), i32> {
    let mut handles = Vec::new();
    let mut tid: [libc::pthread_t; FS_CHN_NUM] = [0; FS_CHN_NUM];
    let mut ret: i32;
    for i in 0..FS_CHN_NUM {
        if unsafe { CHN[i].enable } {
            let mut arg = if unsafe { CHN[i].payload_type } == PT_JPEG {
                (unsafe { CHN[i].payload_type as i32 } << 16) | (3 + unsafe { CHN[i].index })
            } else {
                (unsafe { CHN[i].payload_type as i32 } << 16) | unsafe { CHN[i].index }
            };

            #[cfg(feature = "glibc")]
            {
                let handle = thread::spawn(move || get_video_stream(&mut arg));
            }
            #[cfg(not(feature = "glibc"))]
            {
                unsafe {
                    let handle = libc::pthread_create(
                        &mut tid[i],
                        ptr::null(),
                        libc_get_video_stream,
                        &mut arg as *mut i32 as *mut libc::c_void,
                    );
                }
            }
            unsafe {
                let test =
                    thread::Builder::new().spawn_unchecked(move || get_video_stream(&mut arg));
            }

            handles.push(tid[i]);
        }
    }

    for handle in handles {
        unsafe {
            libc::pthread_join(handle, ptr::null_mut());
        }
    }
    // for handle in handles {
    //     match handle.join() {
    //         Ok(_) => {}
    //         Err(_) => {
    //             imp_log_err!(&TAG, "get_video_stream failed, program will stop !");
    //             return Err(-1);
    //         }
    //     }
    // }

    Ok(())
}

pub fn sample_get_video_stream_byfd() -> Result<(), i32> {
    let mut stream_fd: [Option<File>; FS_CHN_NUM] = Default::default();
    let mut venc_fd: [RawFd; FS_CHN_NUM] = [0; FS_CHN_NUM];
    let mut max_venc_fd = 0;
    let mut total_save_stream_cnt: [i32; FS_CHN_NUM] = [0; FS_CHN_NUM];
    let mut save_stream_cnt: [i32; FS_CHN_NUM] = [0; FS_CHN_NUM];
    let mut stream_path: Vec<String> = vec![String::new(); FS_CHN_NUM];
    let mut read_fds = FdSet::new();
    let mut ret: i32;

    // let mut stream_path: [MaybeUninit<[u8; 128]>; FS_CHN_NUM] = MaybeUninit::uninit_array();
    // let venc_fd: Vec<i32> = vec![]; // 假设你有一些文件描述符
    // let max_venc_fd = venc_fd.iter().map(|&fd| fd).max().unwrap_or(0);
    for i in 0..FS_CHN_NUM {
        save_stream_cnt[i] = 0;

        if unsafe { CHN[i].enable } {
            let chn_num;
            if unsafe { CHN[i].payload_type } == PT_JPEG {
                chn_num = 3 + unsafe { CHN[i].index };
                total_save_stream_cnt[i] = if NR_FRAMES_TO_SAVE / 50 > 0 {
                    NR_FRAMES_TO_SAVE / 50
                } else {
                    NR_FRAMES_TO_SAVE
                };
            } else {
                chn_num = unsafe { CHN[i].index };
                total_save_stream_cnt[i] = NR_FRAMES_TO_SAVE;
            }

            let stream_suffix = match unsafe { CHN[i].payload_type } {
                PT_H264 => "h264",
                PT_H265 => "h265",
                PT_JPEG => "jpeg",
            };

            stream_path[i] = format!(
                "{}/stream-{}.{}",
                STREAM_FILE_PATH_PREFIX, chn_num, stream_suffix
            );

            if unsafe { CHN[i].payload_type } != PT_JPEG {
                stream_fd[i] = Some(
                    OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .mode(0o777)
                        .open(&stream_path[i])
                        .map_err(|e| {
                            imp_log_err!(&TAG, "open stream file error: {}", e);
                            -1
                        })?,
                );
            }

            venc_fd[i] = unsafe { IMP_Encoder_GetFd(chn_num) };
            ret_verify!(venc_fd[i], "IMP_Encoder_GetFd({}) failed\n", chn_num);

            if max_venc_fd < venc_fd[i] {
                max_venc_fd = venc_fd[i];
            }

            ret = unsafe { IMP_Encoder_StartRecvPic(chn_num) };
            ret_verify!(ret, "IMP_Encoder_StartRecvPic({}) failed\n", chn_num);
        }
    }

    loop {
        if save_stream_cnt
            .iter()
            .zip(total_save_stream_cnt.iter())
            .all(|(saved, total)| *saved >= *total)
        {
            break; // 保存帧数足够
        }
        // let mut break_flag = true;
        // for i in 0..FS_CHN_NUM {
        //     break_flag &= save_stream_cnt[i] >= total_save_stream_cnt[i];
        // }
        // if break_flag {
        //     break; // 保存帧数足够
        // }
        read_fds.clear();
        for i in 0..FS_CHN_NUM {
            if unsafe { CHN[i].enable } && save_stream_cnt[i] < total_save_stream_cnt[i] {
                //let mut raw_fd: RawFd = venc_fd[i].as_raw_fd();
                //raw_fd
                read_fds.insert(unsafe { BorrowedFd::borrow_raw(venc_fd[i]) });
            }
        }

        let mut select_timeout = TimeVal::new(2, 0);

        ret = select(
            max_venc_fd + 1,
            &mut read_fds,
            None,
            None,
            Some(&mut select_timeout),
        )
        .map_err(|e| e as i32)?;
        if ret < 0 {
            imp_log_err!(&TAG, "select failed");
            return Err(ret);
        } else if ret == 0 {
            continue;
        } else {
            for i in 0..FS_CHN_NUM {
                if unsafe { CHN[i].enable && read_fds.contains(BorrowedFd::borrow_raw(venc_fd[i])) }
                {
                    let mut stream: IMPEncoderStream = Default::default();

                    let chn_num = if unsafe { CHN[i].payload_type } == PT_JPEG {
                        3 + unsafe { CHN[i].index }
                    } else {
                        unsafe { CHN[i].index }
                    };

                    /* Get H264 or H265 Stream */
                    ret = unsafe { IMP_Encoder_GetStream(chn_num, &mut stream, true) };
                    ret_verify!(ret, "IMP_Encoder_GetStream({}) failed", chn_num);

                    if unsafe { CHN[i].payload_type } == PT_JPEG {
                        if let Err(ret) =
                            save_stream_by_name(&stream_path[i], save_stream_cnt[i], &stream)
                        {
                            return Err(-1);
                        };
                    } else {
                        if let Some(ref mut file) = stream_fd[i] {
                            if let Err(ret) = save_stream(file, &stream) {
                                return Err(-1);
                            };
                        }
                    }

                    unsafe {
                        IMP_Encoder_ReleaseStream(chn_num, &mut stream);
                    }
                    save_stream_cnt[i] += 1;
                }
            }
        }
    }

    for i in 0..FS_CHN_NUM {
        if unsafe { CHN[i].enable } {
            let chn_num = if unsafe { CHN[i].payload_type } == PT_JPEG {
                3 + unsafe { CHN[i].index }
            } else {
                unsafe { CHN[i].index }
            };
            unsafe {
                IMP_Encoder_StopRecvPic(chn_num);
                stream_fd[i].take();
            }
        }
    }

    Ok(())
}
