use std::io::{Read, Write};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use lib::*;

fn main() {
    let mut conn = PoolConnection::new(
        "ITA_Miner",
        "eu.stratum.slushpool.com:3333",
        "worker1");

    let mut serial = serialport::new("COM10", 9600).open().expect("Unable to open serial port");

    let (header_tx, header_rx) = mpsc::channel::<Vec<u8>>();
    let (nonce_tx, nonce_rx) = mpsc::channel();

    thread::spawn(move || {
        let mut nonce_bytes = [0u8; 4];
        loop {
            if let Ok(header) = header_rx.try_recv() {
                serial.write(&header).expect("Wrote header to serial");
                println!("{:?}", header);
            }
            if let Ok(bytes) = serial.bytes_to_read() {
                if bytes > 0 {
                    println!("{} available bytes", bytes);
                }

                if bytes == 4 {
                    serial.read(&mut nonce_bytes).expect("Read data.");
                    let nonce = (nonce_bytes[0] as u32) | ((nonce_bytes[1] as u32) << 8) | ((nonce_bytes[2] as u32) << 16) | ((nonce_bytes[3] as u32) << 24);
                    println!("Found nonce {}", nonce);
                    // nonce_tx.send(nonce).expect("Sent nonce");
                }
            }
        }
    });

    conn.init_connection();

    let mut current_job = None;
    loop {
        let job = conn.handle_datastream();

        if let Some(job) = job {
            let extranonce2 = extranonce2(&job.extranonce2);
            let coinbase = build_coinbase(&job.coinb1, &job.coinb2, &job.extranonce1, &extranonce2);
            let merkle_root = build_root(&job.merkle_branch, &coinbase);
            let header = build_header(&job.version, &revec(&job.prev_block_hash), &merkle_root, &job.ntime, &u32_u8(&job.nbits), &0);

            // 42a14695
            let header = vec![
                0x01, 0x00, 0x00, 0x00, 0x81, 0xcd, 0x02, 0xab,
                0x7e, 0x56, 0x9e, 0x8b, 0xcd, 0x93, 0x17, 0xe2,
                0xfe, 0x99, 0xf2, 0xde, 0x44, 0xd4, 0x9a, 0xb2,
                0xb8, 0x85, 0x1b, 0xa4, 0xa3, 0x08, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0xe3, 0x20, 0xb6, 0xc2,
                0xff, 0xfc, 0x8d, 0x75, 0x04, 0x23, 0xdb, 0x8b,
                0x1e, 0xb9, 0x42, 0xae, 0x71, 0x0e, 0x95, 0x1e,
                0xd7, 0x97, 0xf7, 0xaf, 0xfc, 0x88, 0x92, 0xb0,
                0xf1, 0xfc, 0x12, 0x2b, 0xc7, 0xf5, 0xd7, 0x4d,
                0xf2, 0xb9, 0x44, 0x1a, 0x42, 0xa1, 0x46, 0x01];

            println!("{:?}", header);
            header_tx.send(header).expect("Sent data to Serial Thread.");
            current_job = Some((job, extranonce2));
        }

        if let Ok(nonce) = nonce_rx.try_recv() {
            let current = current_job.unwrap();
            let job = current.0;
            conn.submit_share(&job.job_id, current.1, &job.ntime, nonce);
            current_job = None;
        }
    }
}
