`include "tlhost.svh"
module tb;
  `TB_COMMON
  tlul_cmd_intg_gen u_gen (.tl_i(h_raw), .tl_o(h));
  logic [1:0] intr; integer k; logic [63:0] sdig; csrng_pkg::csrng_req_t cmd;
  edn dut (.clk_i(clk), .rst_ni(rst_n), .tl_i(h), .tl_o(d), .edn_i('0), .edn_o(),
    .csrng_cmd_o(cmd), .csrng_cmd_i('0), .alert_rx_i({2{4'b0101}}), .alert_tx_o(),
    .intr_edn_cmd_req_done_o(intr[0]), .intr_edn_fatal_err_o(intr[1]));
  always @(posedge clk) if (!rst_n) sdig <= 64'h0; else sdig <= {sdig[62:0], sdig[63]} ^ 64'({intr, cmd});
  initial begin
    sdig = 64'h0;
    `TB_RESET
    `TB_SWEEP(32'h44)
    repeat (200) @(posedge clk);
    for (k = 0; k <= 32'h44; k = k + 4) `TLR(32'(k))
    $display("S sdig=%h intr=%b", sdig, intr);
    `TB_END
  end
  initial begin #3000000 $display("WATCHDOG"); $finish; end
endmodule
