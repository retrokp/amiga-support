//! Unofficial support functions for Amiga (m68k) system libraries.

// implementations for functions in amiga.lib, because these are not present
// in the Amiga kernel system libraries

#![feature(asm_experimental_arch)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![no_std]

use amiga_sys::{
    APTR, BOOL, CONST_STRPTR, CxObj, FLOAT, Hook, IClass, IORequest, IOStdReq, InputEvent, Isrvstr,
    KeyMap, LONG, Library, List, Msg, MsgPort, Object, RexxMsg, STRPTR, Task, TimeVal_Type, ULONG,
    UWORD,
};
use core::arch::asm;
use core::ffi::c_void;

// exec support functions

pub unsafe fn BeginIO(ioReq: *mut IORequest) {
    unsafe {
        asm!(
            ".short 0x48e7", // movem.l %d0-%d1/%a0-%a1, -(%sp)
            ".short 0xc0c0",
            "move.l %a6, -(%sp)",
            "move.l 20(%a1), %a6",
            ".short 0x4eae", // jsr -30(a6)
            ".short -30",
            "move.l (%sp)+, %a6",
            "movem.l (%sp)+, %d0-%d1/%a0-%a1",
            in("a1") ioReq,
        );
    }
}

pub unsafe fn CreateExtIO(
    execlib: *mut Library,
    port: *mut MsgPort,
    ioSize: LONG,
) -> *mut IORequest {
    if port == core::ptr::null_mut() || ioSize <= 0 || ioSize > u16::MAX as i32 {
        return core::ptr::null_mut();
    }
    unsafe {
        let io_req = amiga_sys::AllocMem(
            execlib,
            ioSize as u32,
            amiga_sys::MEMF_CLEAR | amiga_sys::MEMF_PUBLIC,
        ) as *mut IORequest;
        if io_req == core::ptr::null_mut() {
            return core::ptr::null_mut();
        }
        (*io_req).io_Message.mn_Node.ln_Type = amiga_sys::NT_MESSAGE as u8;
        (*io_req).io_Message.mn_Length = ioSize as u16;
        (*io_req).io_Message.mn_ReplyPort = port;
        io_req
    }
}

/// TODO
pub unsafe fn CreatePort(execlib: *mut Library, name: CONST_STRPTR, pri: LONG) -> *mut MsgPort {
    unsafe {
        let sig_bit = amiga_sys::AllocSignal(execlib, -1);
        if sig_bit == -1 {
            return core::ptr::null_mut();
        }
        // Rust compiler bugs: it crashes if these are enabled
        let port = core::ptr::null_mut(); /*amiga_sys::AllocMem(execlib,
        core::mem::size_of::<amiga_sys::MsgPort>() as u32,
        amiga_sys::MEMF_CLEAR | amiga_sys::MEMF_PUBLIC);*/
        /*
        if port == core::ptr::null_mut() {
            amiga_sys::FreeSignal(execlib, sig_bit as i32);
            return core::ptr::null_mut();
        }
        */
        let port = port as *mut MsgPort;
        /*
        (*port).mp_Node.ln_Name = name as *mut i8;
        (*port).mp_Node.ln_Pri = pri as i8;
        (*port).mp_Node.ln_Type = amiga_sys::NT_MSGPORT as u8;
        (*port).mp_Flags = amiga_sys::PA_SIGNAL as u8;
        (*port).mp_SigBit = sig_bit as u8;
        (*port).mp_SigTask = amiga_sys::FindTask(execlib, core::ptr::null()) as *mut c_void;
        if name != core::ptr::null_mut() {
            amiga_sys::AddPort(execlib, port);
        } else {
            NewList(&mut ((*port).mp_MsgList) as *mut amiga_sys::List);
        }
        */
        port
    }
}

pub unsafe fn CreateStdIO(execlib: *mut Library, port: *mut MsgPort) -> *mut IOStdReq {
    unsafe { CreateExtIO(execlib, port, core::mem::size_of::<IOStdReq>() as LONG) as *mut IOStdReq }
}

const ME_TASK: usize = 0;
const ME_STACK: usize = 1;
const NUMENTRIES: usize = 2;

#[allow(dead_code)]
struct FakeMemEntry {
    fme_reqs: u32,
    fme_length: u32,
}

#[allow(dead_code)]
struct FakeMemList {
    fml_node: amiga_sys::Node,
    fml_num_entries: usize,
    fml_me: [FakeMemEntry; NUMENTRIES],
}

pub unsafe fn CreateTask(
    execlib: *mut Library,
    name: CONST_STRPTR,
    pri: LONG,
    initPC: APTR,
    mut stackSize: ULONG,
) -> *mut Task {
    // TODO: check bounds
    let pri = pri as i8;

    stackSize = (stackSize + 3) & !3; // round up to multiple of 4
    let fake_mem_list = FakeMemList {
        fml_node: amiga_sys::Node {
            ln_Succ: core::ptr::null_mut(),
            ln_Pred: core::ptr::null_mut(),
            ln_Type: 0,
            ln_Pri: 0,
            ln_Name: core::ptr::null_mut(),
        },
        fml_num_entries: NUMENTRIES,
        fml_me: [
            FakeMemEntry {
                fme_reqs: amiga_sys::MEMF_PUBLIC | amiga_sys::MEMF_CLEAR,
                fme_length: core::mem::size_of::<amiga_sys::Task>() as u32,
            },
            FakeMemEntry {
                fme_reqs: amiga_sys::MEMF_CLEAR,
                fme_length: stackSize,
            },
        ],
    };
    unsafe {
        // may leak memory: If any one of the allocations fails, this function fails to
        // back out fully(?)
        let ml = amiga_sys::AllocEntry(
            execlib,
            (&fake_mem_list) as *const FakeMemList as *const amiga_sys::MemList,
        );
        // AllocEntry() sets bit 31 on failure
        if ml as u32 & 0x8000_0000 == 0x8000_0000 {
            return core::ptr::null_mut();
        }
        let new_task = (*ml).ml_ME[ME_TASK].me_Un.meu_Addr as *mut amiga_sys::Task;
        // TODO: in Rust, a pointer outside array bounds is undefined behavior: fix this?
        let stackentry = (*ml).ml_ME.as_ptr().wrapping_add(ME_STACK);
        (*new_task).tc_SPLower = (*stackentry).me_Un.meu_Addr; // (*ml).ml_ME[ME_STACK].me_Un.meu_Addr;
        (*new_task).tc_SPUpper = (*new_task).tc_SPLower.wrapping_add(stackSize as usize);
        (*new_task).tc_SPReg = (*new_task).tc_SPUpper;
        (*new_task).tc_Node.ln_Type = amiga_sys::NT_TASK as u8;
        (*new_task).tc_Node.ln_Pri = pri;
        (*new_task).tc_Node.ln_Name = name as *mut i8;
        NewList(&mut (*new_task).tc_MemEntry);
        amiga_sys::AddHead(
            execlib,
            &mut (*new_task).tc_MemEntry,
            ml as *mut amiga_sys::Node,
        );
        let res = amiga_sys::AddTask(execlib, new_task, initPC, core::ptr::null_mut());
        // for V37 and later: if AddTask returns null (error), free memory and return null
        // for V36 and earlier, AddTask() doesn't return any value
        if (*execlib).lib_Version >= 37 {
            if res == core::ptr::null_mut() {
                amiga_sys::FreeEntry(execlib, ml);
                return core::ptr::null_mut();
            }
        }
        new_task
    }
}

pub unsafe fn DeleteExtIO(execlib: *mut Library, ioReq: *mut IORequest) {
    if ioReq == core::ptr::null_mut() {
        return;
    }
    unsafe {
        // invalidate structure fields to make detecting use-after-free easier
        (*ioReq).io_Message.mn_Node.ln_Type = 0xff;
        (*ioReq).io_Device = usize::MAX as *mut amiga_sys::Device;
        (*ioReq).io_Unit = usize::MAX as *mut amiga_sys::Unit;
        amiga_sys::FreeMem(
            execlib,
            ioReq as *mut c_void,
            (*ioReq).io_Message.mn_Length as u32,
        );
    }
}

pub unsafe fn DeletePort(execlib: *mut Library, port: *mut MsgPort) {
    unsafe {
        if (*port).mp_Node.ln_Name != core::ptr::null_mut() {
            amiga_sys::RemPort(execlib, port);
        }
        // invalidate structure fields to make detecting use-after-free easier
        (*port).mp_Node.ln_Type = 0xff;
        (*port).mp_MsgList.lh_Head = usize::MAX as *mut amiga_sys::Node;
        amiga_sys::FreeSignal(execlib, (*port).mp_SigBit as i32);
        amiga_sys::FreeMem(
            execlib,
            port as *mut c_void,
            core::mem::size_of::<amiga_sys::MsgPort>() as u32,
        );
    }
}

pub unsafe fn DeleteStdIO(execlib: *mut Library, ioReq: *mut IOStdReq) {
    unsafe {
        DeleteExtIO(execlib, ioReq as *mut amiga_sys::IORequest);
    }
}

pub unsafe fn DeleteTask(execlib: *mut Library, task: *mut Task) {
    unsafe {
        amiga_sys::RemTask(execlib, task);
    }
}

pub unsafe fn NewList(list: *mut List) {
    unsafe {
        (*list).lh_Head =
            &mut ((*list).lh_Tail) as *mut *mut amiga_sys::Node as *mut amiga_sys::Node;
        (*list).lh_Tail = core::ptr::null_mut();
        (*list).lh_TailPred =
            &mut ((*list).lh_Head) as *mut *mut amiga_sys::Node as *mut amiga_sys::Node;
    }
}

/// TODO
pub unsafe fn LibAllocPooled(execlib: *mut Library, poolHeader: APTR, memSize: ULONG) -> APTR {
    unimplemented!();
}

/// TODO
pub unsafe fn LibCreatePool(
    execlib: *mut Library,
    memFlags: ULONG,
    puddleSize: ULONG,
    threshSize: ULONG,
) -> APTR {
    unimplemented!();
}

/// TODO
pub unsafe fn LibDeletePool(execlib: *mut Library, poolHeader: APTR) {
    unimplemented!();
}

/// TODO
pub unsafe fn LibFreePooled(execlib: *mut Library, poolHeader: APTR, memory: APTR, memSize: ULONG) {
    unimplemented!();
}

// rand support functions

pub unsafe fn FastRand(seed: ULONG) -> ULONG {
    let result: ULONG;
    unsafe {
        asm!(
            "add.l %d0,%d0",
            "bhi 2f",
            "eori.l #0x1d872b41, %d0",
            "2:",
            "rts",
            in("d0") seed,
            lateout("d0") result,
        );
    }
    result
}

/// TODO
pub unsafe fn RangeRand(maxValue: ULONG) -> UWORD {
    unimplemented!();
}

// graphics support functions

/// TODO
pub unsafe fn AddTOF(
    GfxBase: *mut Library,
    i: *mut Isrvstr,
    p: ::core::option::Option<unsafe extern "C" fn(args: APTR) -> LONG>,
    a: APTR,
) {
    unimplemented!();
}

/// TODO
pub unsafe fn RemTOF(GfxBase: *mut Library, i: *mut Isrvstr) {
    unimplemented!();
}

pub unsafe fn waitbeam(GfxBase: *mut Library, b: LONG) {
    unsafe {
        // TODO: check b range, because VBeamPos returns only 0-511 result?
        while amiga_sys::VBeamPos(GfxBase) < b {}
    }
}

// math support functions

/// TODO
pub unsafe fn afp(string: CONST_STRPTR) -> FLOAT {
    unimplemented!();
}

/// TODO
pub unsafe fn arnd(place: LONG, exponent: LONG, string: STRPTR) {
    unimplemented!();
}

/// TODO
pub unsafe fn dbf(exponent_base10: ULONG, mant: ULONG) -> FLOAT {
    unimplemented!();
}

/// TODO
pub unsafe fn fpa(fnum: FLOAT, string: STRPTR) -> LONG {
    unimplemented!();
}

/// TODO
pub unsafe fn fpbcd(fnum: FLOAT, string: STRPTR) {
    unimplemented!();
}

// timer support functions

// minimum Kickstart version 1.2 (V33) ??
/// TODO
pub unsafe fn TimeDelay(unit: LONG, secs: ULONG, microsecs: ULONG) -> LONG {
    unimplemented!();
}

// minimum Kickstart version 2.0 (V36) ??
/// TODO
pub unsafe fn DoTimer(tv: *mut TimeVal_Type, unit: LONG, command: LONG) -> LONG {
    unimplemented!();
}

// commodities support functions

// minimum Kickstart version 2.0 (V36) ??
/// TODO
pub unsafe fn ArgArrayDone() {
    unimplemented!();
}

// minimum Kickstart version 2.0 (V36) ??
/// TODO
pub unsafe fn ArgArrayInit(argc: LONG, argv: *mut CONST_STRPTR) -> *mut STRPTR {
    unimplemented!();
}

// minimum Kickstart version 2.0 (V36) ??
/// TODO
pub unsafe fn ArgInt(tt: *mut CONST_STRPTR, entry: CONST_STRPTR, defaultval: LONG) -> LONG {
    unimplemented!();
}

// minimum Kickstart version 2.0 (V36) ??
/// TODO
pub unsafe fn ArgString(
    tt: *mut CONST_STRPTR,
    entry: CONST_STRPTR,
    defaultstring: CONST_STRPTR,
) -> STRPTR {
    unimplemented!();
}

// minimum Kickstart version 2.0 (V36) ??
/// TODO
pub unsafe fn HotKey(description: CONST_STRPTR, port: *mut MsgPort, id: LONG) -> *mut CxObj {
    unimplemented!();
}

// minimum Kickstart version 2.0 (V36) ??
/// TODO
pub unsafe fn InvertString(str_: CONST_STRPTR, km: *const KeyMap) -> *mut InputEvent {
    unimplemented!();
}

// minimum Kickstart version 2.0 (V36) ??
/// TODO
pub unsafe fn FreeIEvents(events: *mut InputEvent) {
    unimplemented!();
}

// arexx support functions

/// TODO
pub unsafe fn CheckRexxMsg(rexxmsg: *const RexxMsg) -> BOOL {
    unimplemented!();
}

/// TODO
pub unsafe fn GetRexxVar(rexxmsg: *const RexxMsg, name: CONST_STRPTR, result: *mut STRPTR) -> LONG {
    unimplemented!();
}

/// TODO
pub unsafe fn SetRexxVar(
    rexxmsg: *mut RexxMsg,
    name: CONST_STRPTR,
    value: CONST_STRPTR,
    length: LONG,
) -> LONG {
    unimplemented!();
}

// intuition and boopsi support functions

// minimum Kickstart version 2.0 (V36) (?)
/// TODO
pub unsafe fn CallHookA(hookPtr: *mut Hook, obj: *mut Object, message: APTR) -> ULONG {
    unimplemented!();
}

/*
// minimum Kickstart version 2.0 (V36) (?)
// NOTE: variadic function: use the replacement CallHookA()
pub unsafe fn CallHook(hookPtr: *mut Hook, obj: *mut Object, ...) -> ULONG {
    unimplemented!();
}
*/

// minimum Kickstart version 2.0 (V36) (?)
/// TODO
pub unsafe fn DoMethodA(obj: *mut Object, message: Msg) -> ULONG {
    unimplemented!();
}

/*
// minimum Kickstart version 2.0 (V36) (?)
// NOTE: variadic function: use the replacement DoMethodA()
pub unsafe fn DoMethod(obj: *mut Object, methodID: ULONG, ...) -> ULONG {
    unimplemented!();
}
*/

// minimum Kickstart version 2.0 (V36) (?)
/// TODO
pub unsafe fn DoSuperMethodA(cl: *mut IClass, obj: *mut Object, message: Msg) -> ULONG {
    unimplemented!();
}

/*
// minimum Kickstart version 2.0 (V36) (?)
// NOTE: variadic function: use the replacement DoSuperMethodA()
pub unsafe fn DoSuperMethod(cl: *mut IClass, obj: *mut Object, methodID: ULONG, ...) -> ULONG {
    unimplemented!();
}
*/

// minimum Kickstart version 2.0 (V36) (?)
/// TODO
pub unsafe fn CoerceMethodA(cl: *mut IClass, obj: *mut Object, message: Msg) -> ULONG {
    unimplemented!();
}

/*
// minimum Kickstart version 2.0 (V36) (?)
// NOTE: variadic function: use the replacement CoerceMethodA()
pub unsafe fn CoerceMethod(cl: *mut IClass, obj: *mut Object, methodID: ULONG, ...) -> ULONG {
    unimplemented!();
}
*/

// minimum Kickstart version 2.0 (V36) (?)
/// TODO
pub unsafe fn HookEntry(hookPtr: *mut Hook, obj: *mut Object, message: APTR) -> ULONG {
    unimplemented!();
}

/*
// minimum Kickstart version 2.0 (V36) (?)
// NOTE: variadic function: use the replacement DoSuperMethod(cl, obj, OM_SET, taglist, NULL);
pub unsafe fn SetSuperAttrs(cl: *mut IClass, obj: *mut Object, tag1: ULONG, ...) -> ULONG {
    unimplemented!();
}
*/

// network/cryptography support functions

// minimum Kickstart version 2.0 (V37/39) (?)
/// TODO
pub unsafe fn ACrypt(buffer: STRPTR, password: CONST_STRPTR, username: CONST_STRPTR) -> STRPTR {
    unimplemented!();
}
