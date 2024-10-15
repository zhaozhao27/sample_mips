use crate::bindings::exit;
use crate::bindings::gBgramap;
use crate::bindings::index;
use crate::bindings::IMP_OSD_ShowRgn;
use crate::bindings::IMP_OSD_UpdateRgnAttrData;
use crate::bindings::EXIT_ERR;
use crate::bindings::IMP_LOG_LEVEL_ERROR;
use crate::bindings::{IMPOSDRgnAttrData, IMPRgnHandle};
use crate::common::OSD_REGION_HEIGHT;
use crate::common::OSD_REGION_WIDTH;
use core::alloc::Layout;
use core::ptr;
use core::ptr::copy_nonoverlapping;
use core::ptr::null_mut;
use heapless::Vec;
use libc::sleep;
use libc::{c_void, localtime, printf, strftime, time, time_t, tm};
const TAG: &str = "Sample-OSD";

const OSD_LETTER_NUM: u32 = 20;

fn osd_show(prHander: &mut [IMPRgnHandle]) -> i32 {
    let mut ret: i32;
    let grpNum: i32 = 0;
    unsafe {
        ret = IMP_OSD_ShowRgn(prHander[0], grpNum, 1);
        ret |= IMP_OSD_ShowRgn(prHander[1], grpNum, 1);
        ret |= IMP_OSD_ShowRgn(prHander[2], grpNum, 1);
        ret |= IMP_OSD_ShowRgn(prHander[3], grpNum, 1);
    };
    if ret != 0 {
        unsafe {
            printf(b"IMP_OSD_ShowRgn() Cover error\n".as_ptr() as *const _);
        }
    }
    ret
}

fn update_thread(p: *mut libc::c_void) {
    let mut ret: i32 = 0;

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

    unsafe {
        printf(b"osd_show called\n".as_ptr() as *const _);
    }
    if ret < 0 {
        unsafe {
            printf(b"osd_show fail\n".as_ptr() as *const _);
            return;
        }
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

                    date_data = gBgramap[index].pdata as *mut _;
                    fontadv = gBgramap[index].width;
                    penpos_t += gBgramap[index].width;
                },
                '-' => unsafe {
                    date_data = gBgramap[10].pdata as *mut _;
                    fontadv = gBgramap[10].width;
                    penpos_t += gBgramap[10].width;
                },
                ' ' => unsafe {
                    date_data = gBgramap[11].pdata as *mut _;
                    fontadv = gBgramap[11].width;
                    penpos_t += gBgramap[11].width;
                },
                ':' => unsafe {
                    date_data = gBgramap[12].pdata as *mut _;
                    fontadv = gBgramap[12].width;
                    penpos_t += gBgramap[12].width;
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

fn sample_osd_start() {
    let mut ret: i32 = 0;
}
