module top;
  localparam int W = 7;
  function automatic logic [7:0] h(input int x);
    localparam int W = 3;
    logic [W:0] t;
    t = '1;
    return t;
  endfunction
  localparam logic [7:0] P = h(0);
  logic [7:0] v;
  initial begin v = h(0); #1 $display("P=%h v=%h", P, v); $finish; end
  initial #50 $finish;
endmodule
