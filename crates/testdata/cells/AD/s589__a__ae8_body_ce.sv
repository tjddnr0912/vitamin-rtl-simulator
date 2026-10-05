package q;
  localparam int W = 4;
  function automatic logic [7:0] h(input int x);
    logic [7:0] t;
    h = W'(t) + 8'd0;
  endfunction
endpackage
module top;
  localparam logic [7:0] P = q::h(2);
  initial begin #1 $display("P=%b", P); $finish; end
  initial #50 $finish;
endmodule
