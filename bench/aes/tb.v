//======================================================================
// tb.v -- vitamin bench harness for secworks/aes (aes_core)
// Deterministic: no $random, no $time in the digest.
// Drives N (plusarg +N=<count>) key-init + encrypt + decrypt round trips,
// chaining the ciphertext into the next block/key, and accumulates one
// 128-bit digest over every produced result.
//======================================================================
`timescale 1ns/1ps
`default_nettype wire

module tb;

  localparam CLK_HALF = 5;

  reg           clk;
  reg           reset_n;
  reg           encdec;
  reg           init;
  reg           next;
  reg           keylen;
  reg [255 : 0] key;
  reg [127 : 0] block;

  wire           ready;
  wire           result_valid;
  wire [127 : 0] result;

  integer       N;
  integer       i;
  reg [127 : 0] digest;
  reg [127 : 0] chain;
  reg [127 : 0] ct;
  reg [127 : 0] pt;
  reg [31  : 0] ictr;

  aes_core dut(
               .clk(clk),
               .reset_n(reset_n),
               .encdec(encdec),
               .init(init),
               .next(next),
               .ready(ready),
               .key(key),
               .keylen(keylen),
               .block(block),
               .result(result),
               .result_valid(result_valid)
              );

  always
    begin
      #CLK_HALF clk = ~clk;
    end

  task wait_ready;
    begin
      while (!ready)
        begin
          #(2 * CLK_HALF);
        end
    end
  endtask

  // watchdog -- fixed, large; never reached by a healthy run
  initial
    begin
      #500000000;
      $display("WATCHDOG");
      $finish;
    end

  initial
    begin
      N = 200;
      if ($value$plusargs("N=%d", N))
        begin
        end

      clk     = 1'b0;
      reset_n = 1'b1;
      encdec  = 1'b0;
      init    = 1'b0;
      next    = 1'b0;
      keylen  = 1'b0;
      key     = 256'h0;
      block   = 128'h0;
      digest  = 128'h0;
      chain   = 128'h0123456789abcdeffedcba9876543210;

      reset_n = 1'b0;
      #(6 * CLK_HALF);
      reset_n = 1'b1;
      #(6 * CLK_HALF);

      for (i = 0; i < N; i = i + 1)
        begin
          ictr   = i;
          keylen = i[0];
          key    = {chain, chain ^ {4{ictr}}};

          // expand the key schedule
          init = 1'b1;
          #(2 * CLK_HALF);
          init = 1'b0;
          #(2 * CLK_HALF);
          wait_ready;

          // encrypt
          encdec = 1'b1;
          block  = chain ^ {96'h0, ictr};
          next   = 1'b1;
          #(2 * CLK_HALF);
          next   = 1'b0;
          #(2 * CLK_HALF);
          wait_ready;
          ct     = result;
          digest = digest ^ ct;

          // decrypt the ciphertext straight back
          encdec = 1'b0;
          block  = ct;
          next   = 1'b1;
          #(2 * CLK_HALF);
          next   = 1'b0;
          #(2 * CLK_HALF);
          wait_ready;
          pt     = result;
          digest = {digest[126:0], digest[127]} ^ pt;

          chain  = ct;
        end

      $display("DIGEST=%032x", digest);
      $finish;
    end

endmodule

`default_nettype wire
