// Benchmark testbench for alexforencich/verilog-i2c (MIT). Written for this benchmark.
// i2c_master and i2c_slave (device address 0x50) on one open-drain bus, driven by an LFSR
// command and data stream. Every bus/status change (sampled at the falling clock edge),
// every byte the master reads and every byte the slave receives is folded into a 64-bit
// rotate-xor digest; the run ends in one DIGEST line. +N=<cycles> sizes the run (default
// 900000); +TRACE also prints each folded event as a line.
`timescale 1ns/1ps
module tb;
parameter FILTER_LEN = 4;
parameter PRESCALE = 2;
integer NCYC;
reg trace;
initial begin
  if (!$value$plusargs("N=%d", NCYC)) NCYC = 900000;
  trace = $test$plusargs("TRACE");
end
reg clk = 1'b0; always #5 clk = ~clk;
reg rst = 1'b1; wire [15:0] presc = PRESCALE;
reg [6:0] c_addr = 0; reg c_start = 0, c_read = 0, c_write = 0, c_wm = 0, c_stop = 0, c_valid = 0; wire c_ready;
reg [7:0] w_tdata = 0; reg w_tvalid = 0, w_tlast = 0; wire w_tready;
wire [7:0] r_tdata; wire r_tvalid, r_tlast; reg r_tready = 0;
wire m_scl_o, m_scl_t, m_sda_o, m_sda_t, m_busy, m_bus_control, m_bus_active, m_missed_ack;
wire s_scl_o, s_scl_t, s_sda_o, s_sda_t, s_busy, s_bus_addressed, s_bus_active;
wire [6:0] s_bus_address;
reg [7:0] sd_tdata = 0; reg sd_tvalid = 0, sd_tlast = 0; wire sd_tready;
wire [7:0] sm_tdata; wire sm_tvalid, sm_tlast; reg sm_tready = 0;
wire scl = (m_scl_t ? 1'b1 : m_scl_o) & (s_scl_t ? 1'b1 : s_scl_o);
wire sda = (m_sda_t ? 1'b1 : m_sda_o) & (s_sda_t ? 1'b1 : s_sda_o);
i2c_master m (.clk(clk), .rst(rst),
  .s_axis_cmd_address(c_addr), .s_axis_cmd_start(c_start), .s_axis_cmd_read(c_read), .s_axis_cmd_write(c_write),
  .s_axis_cmd_write_multiple(c_wm), .s_axis_cmd_stop(c_stop), .s_axis_cmd_valid(c_valid), .s_axis_cmd_ready(c_ready),
  .s_axis_data_tdata(w_tdata), .s_axis_data_tvalid(w_tvalid), .s_axis_data_tready(w_tready), .s_axis_data_tlast(w_tlast),
  .m_axis_data_tdata(r_tdata), .m_axis_data_tvalid(r_tvalid), .m_axis_data_tready(r_tready), .m_axis_data_tlast(r_tlast),
  .scl_i(scl), .scl_o(m_scl_o), .scl_t(m_scl_t), .sda_i(sda), .sda_o(m_sda_o), .sda_t(m_sda_t),
  .busy(m_busy), .bus_control(m_bus_control), .bus_active(m_bus_active), .missed_ack(m_missed_ack),
  .prescale(presc), .stop_on_idle(1'b0));
i2c_slave #(.FILTER_LEN(FILTER_LEN)) s (.clk(clk), .rst(rst), .release_bus(1'b0),
  .s_axis_data_tdata(sd_tdata), .s_axis_data_tvalid(sd_tvalid), .s_axis_data_tready(sd_tready), .s_axis_data_tlast(sd_tlast),
  .m_axis_data_tdata(sm_tdata), .m_axis_data_tvalid(sm_tvalid), .m_axis_data_tready(sm_tready), .m_axis_data_tlast(sm_tlast),
  .scl_i(scl), .scl_o(s_scl_o), .scl_t(s_scl_t), .sda_i(sda), .sda_o(s_sda_o), .sda_t(s_sda_t),
  .busy(s_busy), .bus_address(s_bus_address), .bus_addressed(s_bus_addressed), .bus_active(s_bus_active),
  .enable(1'b1), .device_address(7'h50), .device_address_mask(7'h7f));
reg [31:0] lfsr = 32'hace1_2345; integer cyc = 0; integer ncmd = 0, nrd = 0, nwr = 0;
reg [63:0] dig = 64'd0;
always @(posedge clk) begin
  lfsr <= {lfsr[30:0], lfsr[31] ^ lfsr[21] ^ lfsr[1] ^ lfsr[0]};
  if (cyc == 8) rst <= 1'b0;
  if (c_valid && c_ready) c_valid <= 1'b0;
  if (!rst && (!c_valid || c_ready) && lfsr[4]) begin
    c_addr <= lfsr[9] ? 7'h50 : 7'h51;
    c_start <= lfsr[10] & lfsr[11];
    c_read <= (lfsr[13:12] == 2'd0); c_write <= (lfsr[13:12] == 2'd1); c_wm <= (lfsr[13:12] == 2'd2);
    c_stop <= lfsr[14] | (lfsr[13:12] == 2'd3);
    c_valid <= 1'b1; ncmd <= ncmd + 1;
  end
  if (w_tvalid && w_tready) w_tvalid <= 1'b0;
  if (!rst && (!w_tvalid || w_tready)) begin w_tdata <= lfsr[23:16]; w_tlast <= lfsr[17] & lfsr[18]; w_tvalid <= 1'b1; end
  if (sd_tvalid && sd_tready) sd_tvalid <= 1'b0;
  if (!rst && (!sd_tvalid || sd_tready)) begin sd_tdata <= lfsr[31:24]; sd_tlast <= lfsr[25]; sd_tvalid <= 1'b1; end
  r_tready <= lfsr[6] | lfsr[7];
  sm_tready <= lfsr[8] | lfsr[7];
  if (r_tvalid && r_tready) begin
    if (trace) $display("MRD %0d %h %b", cyc, r_tdata, r_tlast);
    dig = {dig[62:0], dig[63]} ^ {8'hA1, cyc, 15'd0, r_tlast, r_tdata};
    nrd <= nrd + 1;
  end
  if (sm_tvalid && sm_tready) begin
    if (trace) $display("SWR %0d %h %b", cyc, sm_tdata, sm_tlast);
    dig = {dig[62:0], dig[63]} ^ {8'hB2, cyc, 15'd0, sm_tlast, sm_tdata};
    nwr <= nwr + 1;
  end
end
reg [15:0] prev = 16'hffff;
wire [15:0] st = {scl, sda, m_busy, m_bus_control, m_bus_active, m_missed_ack, s_busy, s_bus_addressed, s_bus_active, c_ready, w_tready, sd_tready, 4'd0};
always @(negedge clk) begin
  cyc = cyc + 1;
  if (st !== prev) begin
    if (trace) $display("C %0d %b %h", cyc, st[15:4], s_bus_address);
    dig = {dig[62:0], dig[63]} ^ {8'hC3, cyc, 4'd0, st[15:4], 1'b0, s_bus_address};
    prev = st;
  end
  if (cyc == NCYC) begin
    $display("NCMD=%0d NRD=%0d NWR=%0d", ncmd, nrd, nwr);
    dig = {dig[62:0], dig[63]} ^ {ncmd, nrd};
    dig = {dig[62:0], dig[63]} ^ {32'd0, nwr};
    $display("DIGEST=%h", dig);
    $finish;
  end
end
endmodule
