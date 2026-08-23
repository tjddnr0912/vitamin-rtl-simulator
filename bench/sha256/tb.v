//======================================================================
// tb.v -- vitamin bench harness for secworks/sha256 sha256_core
// Chains N SHA-256/SHA-224 block compressions, feeding each digest
// back in as the next block, and accumulates one DIGEST= line.
//======================================================================
`timescale 1ns/1ps

module tb;

  parameter CLK_HALF_PERIOD = 2;

  reg            tb_clk;
  reg            tb_reset_n;
  reg            tb_init;
  reg            tb_next;
  reg            tb_mode;
  reg [511 : 0]  tb_block;
  wire           tb_ready;
  wire [255 : 0] tb_digest;
  wire           tb_digest_valid;

  integer        N;
  integer        i;
  reg [255 : 0]  acc;
  reg [255 : 0]  chain;

  sha256_core dut(
                  .clk(tb_clk),
                  .reset_n(tb_reset_n),
                  .init(tb_init),
                  .next(tb_next),
                  .mode(tb_mode),
                  .block(tb_block),
                  .ready(tb_ready),
                  .digest(tb_digest),
                  .digest_valid(tb_digest_valid)
                 );

  always
    begin : clk_gen
      #CLK_HALF_PERIOD;
      tb_clk = !tb_clk;
    end

  // Watchdog: deterministic hard stop.
  initial
    begin : watchdog
      #40000000;
      $display("WATCHDOG");
      $finish;
    end

  initial
    begin : main
      tb_clk     = 0;
      tb_reset_n = 1;
      tb_init    = 0;
      tb_next    = 0;
      tb_mode    = 1;
      tb_block   = 512'h0;
      acc        = 256'h0;
      chain      = 256'h0123456789abcdeffedcba987654321000112233445566778899aabbccddeeff;
      N          = 2000;

      if ($value$plusargs("N=%d", N))
        begin
        end

      // reset
      tb_reset_n = 0;
      #(4 * CLK_HALF_PERIOD);
      tb_reset_n = 1;
      @(posedge tb_clk);

      for (i = 0; i < N; i = i + 1)
        begin
          tb_block = {chain, ~chain};
          tb_mode  = i[0];
          tb_init  = 1;
          @(posedge tb_clk);
          tb_init  = 0;
          @(posedge tb_clk);
          while (!tb_ready)
            @(posedge tb_clk);
          acc   = acc ^ tb_digest;
          chain = {tb_digest[191 : 0], tb_digest[255 : 192]} ^ {224'h0, i[31 : 0]};
        end

      $display("DIGEST=%064x", acc);
      $finish;
    end

endmodule
