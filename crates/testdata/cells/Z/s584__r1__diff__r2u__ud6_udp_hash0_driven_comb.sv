primitive inv(o, a); output o; input a; table 0:1; 1:0; endtable endprimitive
module top;
  logic k;
  wire o;
  logic [1:0] y;
  inv u(o, k);
  always_comb y = {o, o};
  initial #0 k = 1'b0;
  initial begin #0; #0 $display("z2 t=%0t o=%b y=%b", $time, o, y); #1 $display("e t=%0t o=%b y=%b", $time, o, y); $finish; end
endmodule
