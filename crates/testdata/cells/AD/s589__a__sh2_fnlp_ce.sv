package q;
  localparam int W = 7;
  function automatic logic [7:0] h(input int x);
    localparam int W = 3;
    logic [W:0] t;
    t = '1;
    return t;
  endfunction
endpackage
module top;
  localparam logic [7:0] P = q::h(0);
  initial begin #1 $display("P=%h", P); $finish; end
  initial #50 $finish;
endmodule
