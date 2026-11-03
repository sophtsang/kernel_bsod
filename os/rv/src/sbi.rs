// calls into OpenSBI (M-mode firmware) via ecall. SBI spec: extension id in a7,
// function id in a6, args in a0.., returns (error in a0, value in a1)
use core::arch::asm;

const EID_TIME: usize = 0x5449_4D45; // "TIME"
const FID_SET_TIMER: usize = 0;

pub struct SbiRet {
    pub error: isize,
    pub value: usize,
}

fn sbi_call(eid: usize, fid: usize, arg0: usize) -> SbiRet {
    let (error, value): (isize, usize);
    unsafe {
        asm!(
            "ecall",
            inlateout("a0") arg0 => error,
            lateout("a1") value,
            in("a6") fid,
            in("a7") eid,
        );
    }
    SbiRet { error, value }
}

// fires a supervisor timer interrupt once `time` >= deadline; also clears a pending one
pub fn set_timer(deadline: u64) -> SbiRet {
    sbi_call(EID_TIME, FID_SET_TIMER, deadline as usize)
}
