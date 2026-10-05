package q;
  function automatic logic [3:0] h(input int x);
    logic [3:0] t;
    h = t;
  endfunction
endpackage
module top;
  localparam logic [3:0] P = q::h(2);
  initial begin #1 $display("P=%b", P); $finish; end
  initial #50 $finish;
endmodule
