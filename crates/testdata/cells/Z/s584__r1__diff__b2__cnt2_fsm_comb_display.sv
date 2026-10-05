module top;
  logic clk, rst_n, go;
  logic [1:0] state, nxt;
  initial begin clk = 1'b0; rst_n = 1'b0; go = 1'b0; #12 rst_n = 1'b1; end
  always #5 clk = ~clk;
  always_ff @(posedge clk or negedge rst_n) if (!rst_n) state <= 2'd0; else state <= nxt;
  always_comb begin nxt = go ? state + 2'd1 : state; $display("C t=%0t state=%b go=%b nxt=%b", $time, state, go, nxt); end
  initial #20 $finish;
endmodule
