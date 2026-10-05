module top;
  logic a = 1'b0;
  logic y;
  wire v;
  assign v = y;
  always_comb y = ~a;
  initial begin #0 $display("z t=%0t v=%b y=%b", $time, v, y); #0 $display("zz t=%0t v=%b y=%b", $time, v, y); end
  initial #1 begin $display("e t=%0t v=%b y=%b", $time, v, y); $finish; end
endmodule
