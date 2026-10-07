// Benchmark testbench for alexforencich/verilog-axis (MIT). Written for this benchmark.
// A 4x4 axis_switch feeding one axis_fifo (frame FIFO) per output. Four LFSR frame sources
// honour valid/ready; every output beat is folded into a per-output accumulator, every
// handshake bit into a per-cycle rotate-xor, and the run ends in one DIGEST line.
// +N=<frames per source> sizes the run (default 3000).
`timescale 1ns/1ps
module tb;
parameter S = 4, M = 4, DW = 64, KW = DW/8, IDW = 4, DESTW = 2, UW = 2;
integer NF;
initial if (!$value$plusargs("N=%d", NF)) NF = 3000;
localparam MIDW = IDW + $clog2(S);
localparam SDESTW = DESTW + $clog2(M);
reg clk = 1'b0; always #5 clk = ~clk;
reg rst = 1'b1;
reg  [S*DW-1:0] s_tdata = 0; reg [S*KW-1:0] s_tkeep = 0; reg [S-1:0] s_tvalid = 0, s_tlast = 0;
wire [S-1:0] s_tready;
reg  [S*IDW-1:0] s_tid = 0; reg [S*SDESTW-1:0] s_tdest = 0; reg [S*UW-1:0] s_tuser = 0;
wire [M*DW-1:0] x_tdata; wire [M*KW-1:0] x_tkeep; wire [M-1:0] x_tvalid, x_tlast; wire [M-1:0] x_tready;
wire [M*MIDW-1:0] x_tid; wire [M*DESTW-1:0] x_tdest; wire [M*UW-1:0] x_tuser;
axis_switch #(.S_COUNT(S), .M_COUNT(M), .DATA_WIDTH(DW), .KEEP_ENABLE(1), .KEEP_WIDTH(KW), .ID_ENABLE(1),
  .S_ID_WIDTH(IDW), .M_ID_WIDTH(MIDW), .M_DEST_WIDTH(DESTW), .S_DEST_WIDTH(SDESTW), .USER_ENABLE(1), .USER_WIDTH(UW),
  .UPDATE_TID(1), .S_REG_TYPE(1), .M_REG_TYPE(2), .ARB_TYPE_ROUND_ROBIN(1), .ARB_LSB_HIGH_PRIORITY(1)) sw (
  .clk(clk), .rst(rst),
  .s_axis_tdata(s_tdata), .s_axis_tkeep(s_tkeep), .s_axis_tvalid(s_tvalid), .s_axis_tready(s_tready), .s_axis_tlast(s_tlast),
  .s_axis_tid(s_tid), .s_axis_tdest(s_tdest), .s_axis_tuser(s_tuser),
  .m_axis_tdata(x_tdata), .m_axis_tkeep(x_tkeep), .m_axis_tvalid(x_tvalid), .m_axis_tready(x_tready), .m_axis_tlast(x_tlast),
  .m_axis_tid(x_tid), .m_axis_tdest(x_tdest), .m_axis_tuser(x_tuser));
wire [M*DW-1:0] o_tdata; wire [M*KW-1:0] o_tkeep; wire [M-1:0] o_tvalid, o_tlast; reg [M-1:0] o_tready = 0;
wire [M*MIDW-1:0] o_tid; wire [M*DESTW-1:0] o_tdest; wire [M*UW-1:0] o_tuser;
genvar g;
generate for (g = 0; g < M; g = g + 1) begin : of
  axis_fifo #(.DEPTH(256), .DATA_WIDTH(DW), .KEEP_ENABLE(1), .KEEP_WIDTH(KW), .LAST_ENABLE(1), .ID_ENABLE(1), .ID_WIDTH(MIDW),
    .DEST_ENABLE(1), .DEST_WIDTH(DESTW), .USER_ENABLE(1), .USER_WIDTH(UW), .FRAME_FIFO(1), .DROP_BAD_FRAME(1),
    .USER_BAD_FRAME_VALUE(2'b01), .USER_BAD_FRAME_MASK(2'b01), .DROP_OVERSIZE_FRAME(1)) f (
    .clk(clk), .rst(rst),
    .s_axis_tdata(x_tdata[g*DW +: DW]), .s_axis_tkeep(x_tkeep[g*KW +: KW]), .s_axis_tvalid(x_tvalid[g]), .s_axis_tready(x_tready[g]),
    .s_axis_tlast(x_tlast[g]), .s_axis_tid(x_tid[g*MIDW +: MIDW]), .s_axis_tdest(x_tdest[g*DESTW +: DESTW]), .s_axis_tuser(x_tuser[g*UW +: UW]),
    .m_axis_tdata(o_tdata[g*DW +: DW]), .m_axis_tkeep(o_tkeep[g*KW +: KW]), .m_axis_tvalid(o_tvalid[g]), .m_axis_tready(o_tready[g]),
    .m_axis_tlast(o_tlast[g]), .m_axis_tid(o_tid[g*MIDW +: MIDW]), .m_axis_tdest(o_tdest[g*DESTW +: DESTW]), .m_axis_tuser(o_tuser[g*UW +: UW]),
    .pause_req(1'b0), .pause_ack(), .status_depth(), .status_depth_commit(), .status_overflow(), .status_bad_frame(), .status_good_frame());
end endgenerate
reg [63:0] lfsr [0:S-1]; integer i, frames [0:S-1]; integer cyc = 0;
reg [63:0] acc [0:M-1]; reg [63:0] cycd = 0; integer beats = 0, xc = 0;
reg [63:0] dig;
initial for (i = 0; i < S; i = i + 1) begin lfsr[i] = 64'h9e3779b97f4a7c15 ^ (i * 64'h1000_0001); frames[i] = 0; end
initial for (i = 0; i < M; i = i + 1) acc[i] = 0;
function [63:0] xs; input [63:0] x; reg [63:0] t; begin t = x ^ (x << 13); t = t ^ (t >> 7); t = t ^ (t << 17); xs = t; end endfunction
always @(posedge clk) begin
  if (cyc == 10) rst <= 1'b0;
  for (i = 0; i < S; i = i + 1) begin
    lfsr[i] = xs(lfsr[i]);
    if (s_tvalid[i] && s_tready[i]) begin
      s_tvalid[i] <= 1'b0;
      if (s_tlast[i]) frames[i] = frames[i] + 1;
    end
    if (!rst && (!s_tvalid[i] || s_tready[i]) && frames[i] < NF && lfsr[i][1:0] != 0) begin
      s_tdata[i*DW +: DW] <= {DW/64{lfsr[i]}} ^ cyc;
      s_tkeep[i*KW +: KW] <= lfsr[i][3] ? {KW{1'b1}} : lfsr[i][KW+8:9];
      s_tlast[i] <= (lfsr[i][7:5] == 0);
      s_tid[i*IDW +: IDW] <= lfsr[i][IDW+19:20];
      s_tdest[i*SDESTW +: SDESTW] <= lfsr[i][SDESTW+29:30];
      s_tuser[i*UW +: UW] <= lfsr[i][41:40] & {1'b1, lfsr[i][7:5] == 0 && lfsr[i][44:42] == 0};
      s_tvalid[i] <= 1'b1;
    end
  end
  for (i = 0; i < M; i = i + 1) begin
    o_tready[i] <= lfsr[0][50+i] | lfsr[1][55+i];
    if (o_tvalid[i] && o_tready[i]) begin
      acc[i] <= {acc[i][62:0], acc[i][63]} ^ o_tdata[i*DW +: DW] ^ {o_tkeep[i*KW +: KW], o_tlast[i], o_tid[i*MIDW +: MIDW], o_tdest[i*DESTW +: DESTW], o_tuser[i*UW +: UW]};
      beats = beats + 1;
    end
  end
end
always @(negedge clk) begin
  cyc = cyc + 1;
  if (^{s_tready, x_tvalid, x_tready, o_tvalid, o_tlast} === 1'bx) begin xc = xc + 1; cycd = {cycd[62:0], cycd[63]} ^ 64'h5555; end
  else cycd = {cycd[62:0], cycd[63]} ^ {s_tready, x_tvalid, x_tready, o_tvalid, o_tlast};
  if (cyc % 1000 == 0) $display("CP %0d %h %0d", cyc, cycd, beats);
  if ((frames[0] >= NF && frames[1] >= NF && frames[S-1] >= NF && o_tvalid == 0 && cyc > 100 && x_tvalid == 0) || cyc == NF * 400 + 5000) begin
    for (i = 0; i < M; i = i + 1) $display("ACC%0d=%h", i, acc[i]);
    $display("CYCD=%h CYCLES=%0d BEATS=%0d XC=%0d", cycd, cyc, beats, xc);
    // The digest folds the per-cycle handshake trace, every output's beat accumulator and
    // the three counters, so it moves with any of them.
    dig = cycd;
    for (i = 0; i < M; i = i + 1) dig = {dig[62:0], dig[63]} ^ acc[i];
    dig = ({dig[62:0], dig[63]} ^ {cyc, beats}) + {32'd0, xc} * 64'd1000003;
    $display("DIGEST=%h", dig);
    $finish;
  end
end
endmodule
