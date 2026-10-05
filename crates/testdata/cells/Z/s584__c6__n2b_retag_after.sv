module top;
  logic clk;
  logic t;
  initial t = 1'b1;
  always @(t) $display("b2 t=%0t clk=%b", $time, clk);
  initial #0 $display("z t=%0t clk=%b", $time, clk);
  always begin clk = 1'b0; #5 clk = 1'b1; #5; end
  initial #12 begin $display("e t=%0t clk=%b", $time, clk); $finish; end
endmodule
