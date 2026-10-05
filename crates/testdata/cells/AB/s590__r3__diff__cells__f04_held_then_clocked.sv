module top;
  logic clk = 0; logic [7:0] cnt = 0;
  function automatic logic [7:0] f(input logic [7:0] x);
    if (x === 8'bz) $display("never");
    f = x + 8'd3;
  endfunction
  wire [7:0] h = f(cnt);
  wire [7:0] r = h ^ 8'h55;
  always #1 clk = ~clk;
  always @(posedge clk) begin cnt <= r; $display("t=%0t cnt=%0d h=%0d r=%0d", $time, cnt, h, r); end
  initial #20 $finish;
endmodule
