use embassy_rp::i2c::{Blocking, I2c};
use embassy_rp::peripherals::I2C1;
use embedded_hal::i2c::{I2c as _, Operation};

type Bus = I2c<'static, I2C1, Blocking>;

#[repr(C)]
pub struct VL53L7CX_Platform {
    pub address: u16,
    pub i2c: *mut core::ffi::c_void,
}

unsafe fn get(p: *mut VL53L7CX_Platform) -> (&'static mut Bus, u8) {
    let (p_ref, bus) = unsafe {
        let p_ref = &*p;
        let bus = &mut *(p_ref.i2c.cast::<Bus>());
        (p_ref, bus)
    };

    let addr = u8::try_from(p_ref.address >> 1).unwrap_or(0);
    (bus, addr)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn VL53L7CX_WrByte(p: *mut VL53L7CX_Platform, reg: u16, value: u8) -> u8 {
    let (bus, addr) = unsafe { get(p) };

    let [reg_hi, reg_lo] = reg.to_be_bytes();

    bus.write(addr, &[reg_hi, reg_lo, value]).map_or(1, |()| 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn VL53L7CX_RdByte(p: *mut VL53L7CX_Platform, reg: u16, out: *mut u8) -> u8 {
    let (bus, addr) = unsafe { get(p) };
    let buf = unsafe { core::slice::from_raw_parts_mut(out, 1) };

    bus.write_read(addr, &reg.to_be_bytes(), buf)
        .map_or(1, |()| 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn VL53L7CX_WrMulti(
    p: *mut VL53L7CX_Platform,
    reg: u16,
    data: *const u8,
    size: u32,
) -> u8 {
    let (bus, addr) = unsafe { get(p) };
    let payload = unsafe { core::slice::from_raw_parts(data, size as usize) };

    bus.transaction(
        addr,
        &mut [
            Operation::Write(&reg.to_be_bytes()),
            Operation::Write(payload),
        ],
    )
    .map_or(1, |()| 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn VL53L7CX_RdMulti(
    p: *mut VL53L7CX_Platform,
    reg: u16,
    out: *mut u8,
    size: u32,
) -> u8 {
    let (bus, addr) = unsafe { get(p) };
    let buf = unsafe { core::slice::from_raw_parts_mut(out, size as usize) };

    bus.transaction(
        addr,
        &mut [Operation::Write(&reg.to_be_bytes()), Operation::Read(buf)],
    )
    .map_or(1, |()| 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn VL53L7CX_WaitMs(_p: *mut VL53L7CX_Platform, ms: u32) -> u8 {
    embassy_time::block_for(embassy_time::Duration::from_millis(u64::from(ms)));
    0
}

// Reimplement the byteswap from platform.c — same logic, unaligned-safe
#[unsafe(no_mangle)]
pub const unsafe extern "C" fn VL53L7CX_SwapBuffer(buffer: *mut u8, size: u16) {
    let mut i = 0usize;
    while i < size as usize {
        unsafe {
            let p = buffer.add(i);
            let bytes = p.cast::<[u8; 4]>().read_unaligned();
            let swapped = [bytes[3], bytes[2], bytes[1], bytes[0]];
            p.cast::<[u8; 4]>().write_unaligned(swapped);
        }
        i += 4;
    }
}
