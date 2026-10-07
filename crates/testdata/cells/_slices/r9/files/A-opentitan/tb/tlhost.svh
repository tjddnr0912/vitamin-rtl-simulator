// first-party TL-UL host macros for row9 census testbenches (not upstream)
`define TL_IDLE begin h_raw = tlul_pkg::TL_H2D_DEFAULT; h_raw.a_valid = 1'b0; h_raw.d_ready = 1'b1; end
`define TL_REQ(OP, A, D) begin \
  @(negedge clk); h_raw = tlul_pkg::TL_H2D_DEFAULT; h_raw.a_valid = 1'b1; h_raw.a_opcode = OP; \
  h_raw.a_size = 2'd2; h_raw.a_mask = 4'hf; h_raw.a_address = A; h_raw.a_data = D; h_raw.a_source = 8'd3; h_raw.d_ready = 1'b1; \
  @(posedge clk); while (!d.a_ready) @(posedge clk); \
  @(negedge clk); h_raw.a_valid = 1'b0; \
  while (!d.d_valid) begin @(posedge clk); #1; end \
  rdata = d.d_data; rerr = d.d_error; ntx = ntx + 1; \
  digest = {digest[62:0], digest[63]} ^ {32'(A), rdata} ^ 64'(rerr); \
  $display("T %0d %s a=%h d=%h r=%h e=%b t=%0t", ntx, (OP == tlul_pkg::Get) ? "R" : "W", 32'(A), 32'(D), rdata, rerr, $time); \
end
`define TLW(A, D) `TL_REQ(tlul_pkg::PutFullData, A, D)
`define TLR(A) `TL_REQ(tlul_pkg::Get, A, 32'h0)
`define TB_COMMON \
  logic clk, rst_n; initial begin clk = 1'b0; rst_n = 1'b0; end \
  always #5 clk = ~clk; \
  tlul_pkg::tl_h2d_t h_raw, h; tlul_pkg::tl_d2h_t d; \
  \
  logic [31:0] rdata; logic rerr; integer ntx; logic [63:0] digest; \
  initial begin ntx = 0; digest = 64'h0; end
`define TB_RESET begin `TL_IDLE; rst_n = 1'b0; repeat (5) @(posedge clk); @(negedge clk); rst_n = 1'b1; repeat (3) @(posedge clk); end
`define TB_END begin $display("DIGEST=%h ntx=%0d t=%0t", digest, ntx, $time); $finish; end
`define TB_SWEEP(LAST) begin \
  for (k = 0; k <= LAST; k = k + 4) `TLR(32'(k)) \
  for (k = 0; k <= LAST; k = k + 4) `TLW(32'(k), 32'h5a5a_5a5a ^ 32'(k * 32'h01010101)) \
  for (k = 0; k <= LAST; k = k + 4) `TLR(32'(k)) \
end
