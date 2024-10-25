use libc;
use metrics::metrics_server::{Metrics, MetricsServer};
use metrics::{MetricsRequest, MetricsResponse};
use procfs::process::Process;
use std::io::Error as IoError;
use std::process::{Child, Command, Stdio};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use std::{fs, thread, time};
use tokio::time::{sleep, Duration};
use tonic::{transport::Server, Request, Response, Status};

pub mod metrics {
    tonic::include_proto!("metrics");
}

// Function to spawn a child process based on the command and arguments from the request
fn spawn_child_process(command: &str, args: &[&str]) -> Result<Child, IoError> {
    let mut cmd = Command::new(command);
    for arg in args {
        cmd.arg(arg);
    }
    let child = cmd.spawn()?;
    Ok(child)
}

// Implement the MetricsService struct
#[derive(Debug, Default)]
pub struct MetricsService {}

fn get_cpu_usage(pid: i32, prev_total_time: u64, start_time: Instant) -> Option<f64> {
    let hz = unsafe { libc::sysconf(libc::_SC_CLK_TCK) as f64 }; // Get clock ticks per second

    // Fetch process stat using procfs
    let process = Process::new(pid).ok()?;
    let stat = process.stat().ok()?;

    // Calculate the total CPU time spent by the process (utime + stime)
    let total_time = stat.utime + stat.stime;

    // Calculate CPU usage as a percentage
    let delta_time = (Instant::now() - start_time).as_secs_f64(); // Elapsed time in seconds
    let cpu_usage = ((total_time - prev_total_time) as f64 / hz) / delta_time * 100.0;

    Some(cpu_usage)
}

#[tonic::async_trait]
impl Metrics for MetricsService {
    async fn req_metrics(
        &self,
        request: Request<MetricsRequest>,
    ) -> Result<Response<MetricsResponse>, Status> {
        println!("Got a request: {:?}", request);

        // Extract the command and arguments from the request
        let command_str = &request.into_inner().command;
        let parts: Vec<&str> = command_str.split_whitespace().collect();
        let command = parts
            .get(0)
            .ok_or_else(|| Status::invalid_argument("Command missing"))?;
        let args = &parts[1..];

        // Spawn the child process with the extracted command and arguments
        let mut child = match spawn_child_process(command, args) {
            Ok(child) => child,
            Err(e) => {
                eprintln!("Failed to spawn child process: {}", e);
                return Err(Status::internal("Failed to spawn child process"));
            }
        };

        // Get the child process ID
        let child_pid = child.id() as i32;
        println!("Child process ID: {}", child_pid);

        let start_time = Instant::now();
        let mut prev_total_time = 0;

        let mut cpu_usage_child = 0f64;
        let mut memory_rss_child = 0u64;
        let mut io_bytes_read_child = 0u64;
        let mut io_bytes_written_child = 0u64;
        let mut unix_timestamp = 0u64;

        let now = SystemTime::now();

        // Convert to a UNIX timestamp (seconds since UNIX_EPOCH)
        match now.duration_since(UNIX_EPOCH) {
            Ok(duration) => {
                unix_timestamp = duration.as_secs();
                println!("Current UNIX timestamp: {}", unix_timestamp);
            }
            Err(e) => {
                eprintln!("Error: {:?}", e);
            }
        }

        // Start a task to monitor the metrics every 300ms
        tokio::spawn(async move {
            let process = Process::new(child_pid).expect("Failed to get process info");

            loop {
                // Fetch metrics from /proc
                let stat = process.stat().expect("Failed to get process stat");
                let io = process.io().expect("Failed to get process IO");

                // Calculate memory usage (RSS) in bytes
                memory_rss_child = stat.rss * 4096;
                io_bytes_read_child = io.read_bytes;
                io_bytes_written_child = io.write_bytes;
                // let timestamp = SystemTime::now();

                // Placeholder network bytes (you could replace with actual values)
                // let net_bytes_read = 512;
                // let net_bytes_written = 256;

                // Get CPU usage
                if let Some(cpu_usage_child) = get_cpu_usage(child_pid, prev_total_time, start_time)
                {
                    println!("CPU Usage: {:.2}%", cpu_usage_child);
                } else {
                    println!("Failed to fetch CPU usage.");
                }

                // Print metrics to the terminal
                println!("Memory RSS: {} bytes", memory_rss_child);
                println!("I/O Bytes Read: {}", io_bytes_read_child);
                println!("I/O Bytes Written: {}", io_bytes_written_child);
                // println!("Network Bytes Read: {}", net_bytes_read);
                // println!("Network Bytes Written: {}", net_bytes_written);

                // Wait for 300ms before fetching metrics again
                sleep(Duration::from_millis(300)).await;
            }
        });

        sleep(Duration::from_millis(2000)).await;

        // Return a placeholder response while metrics are being collected
        let reply = MetricsResponse {
            thread_id: child_pid as i64,
            timestamp: unix_timestamp as i64,
            cpu_usage: cpu_usage_child,
            memory_rss: memory_rss_child as i64,
            io_bytes_read: io_bytes_read_child as i64,
            io_bytes_written: io_bytes_written_child as i64,
            net_bytes_read: 512,
            net_bytes_written: 256,
        };

        Ok(Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "0.0.0.0:50051".parse()?;
    let metrics_service = MetricsService::default();

    Server::builder()
        .add_service(MetricsServer::new(metrics_service))
        .serve(addr)
        .await?;

    Ok(())
}
