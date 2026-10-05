package q;
  localparam int W = 31;
  function automatic logic [W:0] h(input int a);
    h = ((0-1)*(0-1)-2)/2;
  endfunction
endpackage
module top;
  localparam logic [31:0] P = q::h(0);
  initial begin #1 $display("P=%h", P); $finish; end
  initial #50 $finish;
endmodule
