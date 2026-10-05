package q;
  localparam int W = 7;
  task automatic t(output logic [7:0] o);
    localparam int W = 3;
    logic [W:0] z;
    z = '1;
    o = z;
  endtask
endpackage
module top;
  logic [7:0] v;
  initial begin q::t(v); $display("v=%h", v); #1 $finish; end
  initial #50 $finish;
endmodule
