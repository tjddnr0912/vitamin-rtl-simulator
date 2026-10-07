// Benchmark testbench for alexforencich/verilog-uart (MIT). Written for this benchmark.
// uart with txd looped back into rxd: LFSR bytes enter the transmitter with a synchronous
// valid/ready handshake at the rising edge, every received byte is folded into an
// accumulator, and seven line/status bits are folded into a per-cycle rotate-xor sampled at
// the falling edge. The run ends in one DIGEST line. +N=<bytes> sizes the run (default
// 20000); +TRACE also prints every received byte and every status change as a line.
`timescale 1ns/1ps
module tb;
parameter DW = 8;
parameter PRESCALE = 1;
integer N;
reg trace;
initial begin
  if (!$value$plusargs("N=%d", N)) N = 20000;
  trace = $test$plusargs("TRACE");
end
reg clk = 1'b0; always #5 clk = ~clk;
reg rst = 1'b1;
reg [DW-1:0] s_tdata = 0; reg s_tvalid = 1'b0; wire s_tready;
wire [DW-1:0] m_tdata; wire m_tvalid; reg m_tready = 1'b0;
wire txd, tx_busy, rx_busy, rx_overrun_error, rx_frame_error;
reg [15:0] prescale = PRESCALE;
uart #(.DATA_WIDTH(DW)) dut (.clk(clk), .rst(rst),
  .s_axis_tdata(s_tdata), .s_axis_tvalid(s_tvalid), .s_axis_tready(s_tready),
  .m_axis_tdata(m_tdata), .m_axis_tvalid(m_tvalid), .m_axis_tready(m_tready),
  .rxd(txd), .txd(txd), .tx_busy(tx_busy), .rx_busy(rx_busy),
  .rx_overrun_error(rx_overrun_error), .rx_frame_error(rx_frame_error), .prescale(prescale));
reg [31:0] lfsr = 32'h1234_5678; integer cyc = 0, sent = 0, got = 0;
reg [63:0] acc = 64'd0, cycd = 64'd0, dig;
always @(posedge clk) begin
  lfsr <= {lfsr[30:0], lfsr[31] ^ lfsr[21] ^ lfsr[1] ^ lfsr[0]};
  if (cyc == 8) rst <= 1'b0;
  if (s_tvalid && s_tready) s_tvalid <= 1'b0;
  if (!rst && (!s_tvalid || s_tready) && sent < N && lfsr[3]) begin
    s_tdata <= lfsr[DW+7:8]; s_tvalid <= 1'b1; sent <= sent + 1;
  end
  m_tready <= lfsr[5] | lfsr[6];
  if (m_tvalid && m_tready) begin
    acc <= {acc[62:0], acc[63]} ^ m_tdata; got <= got + 1;
    if (trace) $display("RX %0d %h", got, m_tdata);
  end
end
reg [7:0] prev = 8'hff;
always @(negedge clk) begin
  cyc = cyc + 1;
  cycd = {cycd[62:0], cycd[63]} ^ {txd, tx_busy, rx_busy, rx_overrun_error, rx_frame_error, m_tvalid, s_tready};
  if ({txd, tx_busy, rx_busy, rx_overrun_error, rx_frame_error, m_tvalid, s_tready} !== prev[6:0]) begin
    if (trace) $display("C %0d %b%b%b%b%b%b%b", cyc, txd, tx_busy, rx_busy, rx_overrun_error, rx_frame_error, m_tvalid, s_tready);
    prev[6:0] = {txd, tx_busy, rx_busy, rx_overrun_error, rx_frame_error, m_tvalid, s_tready};
  end
  if (got == N || cyc == N*(DW+2)*8*PRESCALE*3 + 1000) begin
    $display("SENT=%0d GOT=%0d ACC=%h CYCD=%h CYC=%0d", sent, got, acc, cycd, cyc);
    // The digest folds the received-byte accumulator, the per-cycle status trace and the
    // three counters.
    dig = {cycd[62:0], cycd[63]} ^ acc;
    dig = ({dig[62:0], dig[63]} ^ {sent, got}) + {32'd0, cyc};
    $display("DIGEST=%h", dig);
    $finish;
  end
end
endmodule
