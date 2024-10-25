use metrics::metrics_client::MetricsClient;  // Update to the new Metrics client
use metrics::MetricsRequest;  // Update to the new MetricsRequest
use std::io::{self, Write};
// use tokio::time::{sleep, Duration};

pub mod metrics {
    tonic::include_proto!("metrics");  // Include the generated code from PerformanceMetrics.proto
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to the server at the specified address
    let mut client = MetricsClient::connect("http://127.0.0.1:50051").await?;

    // Prompt user for the command (you can change this to any specific command you expect)
    print!("Enter metrics command: ");
    io::stdout().flush()?;  // Ensure the prompt is shown before user input
    let mut command = String::new();
    io::stdin().read_line(&mut command)?;
    let command = command.trim().to_owned();  // Trim whitespace

    // Create the request with the command input
    let request = tonic::Request::new(
        MetricsRequest {
            command,  // Pass the user input into the request
        }
    );

    // Call the server's req_metrics method and await the response
    let response = client.req_metrics(request).await?;
    
    // Print the response data from the server
    let metrics_response = response.into_inner();

    println!("Thread ID: {}", metrics_response.thread_id);
    println!("Timestamp: {}", metrics_response.timestamp);
    println!("CPU Usage: {}%", metrics_response.cpu_usage);
    println!("Memory RSS: {} bytes", metrics_response.memory_rss);
    println!("I/O Bytes Read: {} bytes", metrics_response.io_bytes_read);
    println!("I/O Bytes Written: {} bytes", metrics_response.io_bytes_written);
    println!("Net Bytes Read: {} bytes", metrics_response.net_bytes_read);
    println!("Net Bytes Written: {} bytes", metrics_response.net_bytes_written);


    Ok(())
}
