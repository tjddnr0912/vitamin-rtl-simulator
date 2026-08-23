// Benchmark testbench for SERV (olofk/serv) -- written for vitamin bench corpus.
// Deterministic, self-terminating, prints exactly one DIGEST line.
// Workload scales with +N=<cycles>.
`define SERV_CLEAR_RAM
`default_nettype none

module tb;

   reg          clk = 1'b0;
   reg          rst = 1'b1;
   wire         q;

   integer      n;
   integer      i;
   reg          plus_ok;
   integer      cyc;
   reg [63:0]   acc;

   servant
     #(.memfile  ("src/sw/blinky.hex"),
       .memsize  (8192),
       .sim      (0),
       .debug    (1'b0),
       .with_csr (1),
       .compress (1'b0),
       .width    (1))
   dut (clk, rst, q);

   always #5 clk = ~clk;

   // Bus taps.  SERV's wishbone lines are only meaningful while stb/ack is
   // asserted; mask them off otherwise so the digest never eats an X.
   wire        ms   = dut.wb_mem_stb;
   wire        ma   = dut.wb_mem_ack;
   wire        es   = dut.wb_ext_stb;
   wire [31:0] madr = ms ? dut.wb_mem_adr : 32'd0;
   wire [31:0] mrdt = ma ? dut.wb_mem_rdt : 32'd0;
   wire        mwe  = ms ? dut.wb_mem_we  : 1'b0;
   wire [31:0] eadr = es ? dut.wb_ext_adr : 32'd0;
   wire [31:0] edat = es ? dut.wb_ext_dat : 32'd0;

   initial begin
      cyc = 0;
      acc = 64'd0;
      plus_ok = $value$plusargs("N=%d", n);
      if (!plus_ok)
        n = 500000;

      rst = 1'b1;
      repeat (16) @(posedge clk);
      rst = 1'b0;
      repeat (4*n + 4000) @(posedge clk);
      $display("WATCHDOG");
      $finish;
   end

   always @(posedge clk) if (!rst) begin
      cyc <= cyc + 1;
      acc <= {acc[62:0], acc[63]}
             ^ {mrdt, madr}
             ^ {edat, eadr}
             ^ {62'd0, mwe, 1'b0};
      if (cyc >= n) begin
         $display("CYCLES=%0d", cyc);
         $display("DIGEST=%h", acc);
         $finish;
      end
   end

endmodule
