package q;
  localparam int W = 3;
  function automatic logic [W:0] h(input int x);
    if (x > 5) h = 4'd1;
  endfunction
endpackage
module top;
  localparam logic [3:0] P = q::h(2);
  initial begin #1 $display("P=%b", P); $finish; end
  initial #50 $finish;
endmodule
