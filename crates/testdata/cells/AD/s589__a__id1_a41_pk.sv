package q;
  localparam int W = 31;
  function automatic logic [W:0] h(input int a);
    int t = int'(2.5);
    t[0] = 1'b0;
    h = t;
  endfunction
endpackage
module top;
  localparam int P = q::h(0);
  initial begin #1 $display("P=%0d", P); $finish; end
  initial #50 $finish;
endmodule
