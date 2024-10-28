use crate::bgramapinfo::BGRA_MAP;
use crate::bindings::exit;
use crate::bindings::index;
use crate::bindings::IMPSensorInfo;
use crate::bindings::IMP_Encoder_CreateGroup;
use crate::bindings::IMP_Encoder_DestroyGroup;
use crate::bindings::IMP_FrameSource_SetFrameDepth;
use crate::bindings::IMP_OSD_CreateGroup;
use crate::bindings::IMP_OSD_ShowRgn;
use crate::bindings::IMP_OSD_UpdateRgnAttrData;
use crate::bindings::IMP_System_Bind;
use crate::bindings::IMP_System_UnBind;
use crate::bindings::EXIT_ERR;
use crate::bindings::{
    IMPCell,
    IMPDeviceID::{self, *},
};
use crate::bindings::{IMPOSDRgnAttrData, IMPRgnHandle};
use crate::common::sample_encoder_exit;
use crate::common::sample_encoder_init;
use crate::common::sample_framesource_exit;
use crate::common::sample_framesource_init;
use crate::common::sample_framesource_streamoff;
use crate::common::sample_framesource_streamon;
use crate::common::sample_get_video_stream;
use crate::common::sample_get_video_stream_byfd;
use crate::common::sample_osd_exit;
use crate::common::sample_osd_init;
use crate::common::sample_system_exit;
use crate::common::sample_system_init;
use crate::common::FS_CHN_NUM;
use crate::common::OSD_REGION_HEIGHT;
use crate::common::OSD_REGION_WIDTH;
use core::alloc::Layout;
use core::mem::size_of;
use core::ptr;
use core::ptr::copy_nonoverlapping;
use core::ptr::null_mut;
use core::slice::from_raw_parts;

use libc::malloc;
use libc::sleep;
use libc::{c_void, free, localtime, printf, strftime, time, time_t, tm};
use libc::{pthread_cancel, pthread_create, pthread_join};
const TAG: &str = "Sample-OSD";
use crate::common::CHN;

const OSD_LETTER_NUM: u32 = 20;
const GRP_NUM: i32 = 0;

fn osd_show(pr_hander: &mut [IMPRgnHandle]) -> i32 {
    let mut ret: i32;
    unsafe {
        ret = IMP_OSD_ShowRgn(pr_hander[0], GRP_NUM, 1);
        ret |= IMP_OSD_ShowRgn(pr_hander[1], GRP_NUM, 1);
        ret |= IMP_OSD_ShowRgn(pr_hander[2], GRP_NUM, 1);
        ret |= IMP_OSD_ShowRgn(pr_hander[3], GRP_NUM, 1);
    };
    if ret != 0 {
        unsafe {
            printf(b"IMP_OSD_ShowRgn() Cover error\n".as_ptr() as *const _);
        }
    }
    ret
}

pub extern "C" fn update_thread(p: *mut c_void) -> *mut c_void {
    let mut ret: i32;

    let mut date_str = [0u8; 40];
    let mut curr_time: time_t;
    let mut curr_date: *mut tm = ptr::null_mut();
    let mut date_data: *mut libc::c_void = ptr::null_mut();
    let data = p as *mut u32;
    let mut r_attr_data: IMPOSDRgnAttrData = Default::default();

    let layout = Layout::array::<IMPRgnHandle>(4).unwrap();
    let mut pr_hander: [IMPRgnHandle; 4] = Default::default();

    let mut i: usize;
    let mut j: u32;

    ret = osd_show(&mut pr_hander);

    imp_log_info!("osd_show called");

    if ret < 0 {
        imp_log_err!("osd_show failed\n");
        return ptr::null::<c_void>() as *mut _;
    }
    loop {
        let mut penpos_t: u32 = 0;
        let mut fontadv: u32 = 0;

        curr_time = unsafe { time(null_mut()) };
        curr_date = unsafe { localtime(&curr_time) };

        unsafe {
            strftime(
                date_str.as_mut_ptr() as *mut i8,
                40,
                b"%Y-%m-%d %I:%M:%S\0".as_ptr() as *const i8,
                curr_date,
            );
        }

        for i in 0..OSD_LETTER_NUM {
            let data_char = date_str[i as usize] as char;
            match data_char {
                '0'..='9' => unsafe {
                    let index = (data_char as u8 - b'0') as usize;

                    date_data = BGRA_MAP[index].pdata as *mut _;
                    fontadv = BGRA_MAP[index].width;
                    penpos_t += BGRA_MAP[index].width;
                },
                '-' => unsafe {
                    date_data = BGRA_MAP[10].pdata as *mut _;
                    fontadv = BGRA_MAP[10].width;
                    penpos_t += BGRA_MAP[10].width;
                },
                ' ' => unsafe {
                    date_data = BGRA_MAP[11].pdata as *mut _;
                    fontadv = BGRA_MAP[11].width;
                    penpos_t += BGRA_MAP[11].width;
                },
                ':' => unsafe {
                    date_data = BGRA_MAP[12].pdata as *mut _;
                    fontadv = BGRA_MAP[12].width;
                    penpos_t += BGRA_MAP[12].width;
                },
                _ => {}
            }

            for j in 0..OSD_REGION_HEIGHT {
                unsafe {
                    #[cfg(feature = "SUPPORT_RGB555LE")]
                    copy_nonoverlapping(
                        (data as *mut u16)
                            .add((j * OSD_LETTER_NUM * OSD_REGION_WIDTH + penpos_t) as usize)
                            as *mut libc::c_void,
                        (date_data as *mut u16).add((j * fontadv) as usize) as *mut libc::c_void,
                        (fontadv as _) * (size_of::<u16>()),
                    );

                    #[cfg(not(feature = "SUPPORT_RGB555LE"))]
                    copy_nonoverlapping(
                        (data as *mut u32)
                            .add((j * OSD_LETTER_NUM * OSD_REGION_WIDTH + penpos_t) as usize)
                            as *mut libc::c_void,
                        (date_data as *mut u32).add((j * fontadv) as usize) as *mut libc::c_void,
                        (fontadv as usize) * (size_of::<u32>()),
                    );
                }
            }
        }

        r_attr_data.picData.pData = data as *mut c_void;
        unsafe {
            IMP_OSD_UpdateRgnAttrData(pr_hander[0], &mut r_attr_data);
            sleep(1);
        }
    }
}

pub fn sample_osd_start() -> Result<(), i32> {
    let mut ret: i32;
    let mut i = 0;
    let mut by_get_fd: i32 = 0;

    let mut pr_hander: *mut IMPRgnHandle = ptr::null::<c_void> as *mut _;
    /* only show OSD in chn0 */
    unsafe {
        CHN[0].enable = true;
        CHN[1].enable = false;
    }

    /* Step.1 System init */
    let mut sensor_info: IMPSensorInfo = Default::default();

    ret = sample_system_init(&mut sensor_info).unwrap();
    ret_verify!(ret, "IMP_System_Init() failed\n");

    /* Step.2 FrameSource init */
    ret = sample_framesource_init().unwrap();
    ret_verify!(ret, "FrameSource init failed\n");
    /* Step.3 Encoder init */
    for i in 0..FS_CHN_NUM {
        unsafe {
            if CHN[i].enable {
                ret = IMP_Encoder_CreateGroup(CHN[i].index);
                ret_verify!(ret, "IMP_Encoder_CreateGroup - osd:192 error !\n")
            }
        }
    }

    sample_encoder_init().unwrap();
    unsafe {
        ret = IMP_OSD_CreateGroup(GRP_NUM);
        ret_verify!(ret, "IMP_OSD_CreateGroup - osd:204 error !\n");
    }

    /* Step.4 OSD init */
    pr_hander = sample_osd_init(GRP_NUM).unwrap();
    // Step 5: Bind
    let mut osd_cell = IMPCell {
        deviceID: crate::bindings::IMPDeviceID::DEV_ID_OSD,
        groupID: GRP_NUM,
        outputID: 0,
    };

    ret = unsafe { IMP_System_Bind(&mut CHN[0].framesource_chn, &mut osd_cell) };
    ret_verify!(ret, "Bind FrameSource channel0 and OSD failed\n\0");

    ret = unsafe { IMP_System_Bind(&mut osd_cell, &mut CHN[0].imp_encoder) };
    ret_verify!(ret, "Bind OSD and Encoder failed");

    // Step 6: Create OSD bgramap update thread
    let time_stamp_data = unsafe {
        #[cfg(feature = "SUPPORT_RGB555LE")]
        {
            malloc(
                (OSD_LETTER_NUM * OSD_REGION_HEIGHT * OSD_REGION_WIDTH) as usize * size_of::<u16>(),
            )
        }
        #[cfg(not(feature = "SUPPORT_RGB555LE"))]
        {
            malloc(
                (OSD_LETTER_NUM * OSD_REGION_HEIGHT * OSD_REGION_WIDTH) as usize * size_of::<u32>(),
            )
        }
    };

    if time_stamp_data.is_null() {
        imp_log_err!("valloc timeStampData error\n\0");
        return Err(-1);
    }

    let mut tid: u32 = 0;
    let ret = unsafe { pthread_create(&mut tid, ptr::null(), update_thread, time_stamp_data) };
    if ret != 0 {
        imp_log_err!("thread create error");
        return Err(-1);
    }

    // Step 7: Stream On
    unsafe {
        IMP_FrameSource_SetFrameDepth(0, 0);
    }
    sample_framesource_streamon().unwrap();

    // Step 6: Get stream
    if by_get_fd != 0 {
        sample_get_video_stream_byfd().unwrap()
    } else {
        sample_get_video_stream().unwrap()
    }

    /* Exit sequence as follow */
    // Step a: Stream Off
    let ret = sample_framesource_streamoff().unwrap();
    unsafe {
        pthread_cancel(tid);
        pthread_join(tid, ptr::null_mut());
        free(time_stamp_data);
    }

    // Step b: UnBind
    let ret = unsafe { IMP_System_UnBind(&mut osd_cell, &mut CHN[0].imp_encoder) };
    ret_verify!(ret, "UnBind OSD and Encoder failed\n");

    let ret = unsafe { IMP_System_UnBind(&mut CHN[0].framesource_chn, &mut osd_cell) };
    ret_verify!(ret, "UnBind FrameSource and OSD failed\n");

    // Step c: OSD exit
    unsafe {
        sample_osd_exit(pr_hander, GRP_NUM).unwrap();
    }

    // Step d: Encoder exit
    sample_encoder_exit().unwrap();

    for ch in unsafe { from_raw_parts(CHN.as_ptr(), CHN.len()) } {
        if ch.enable {
            let ret = unsafe { IMP_Encoder_DestroyGroup(ch.index) };
            ret_verify!(ret, "IMP_Encoder_DestroyGroup failed\n");
        }
    }

    // Step e: FrameSource exit
    sample_framesource_exit().unwrap();

    // Step f: System exit
    sample_system_exit(&mut sensor_info).unwrap();

    imp_log_info!("exit !");
    Ok(())
}
