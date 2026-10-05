module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  logic [7:0] r;
  assign r = ({f(2){1'b1}} + 7'h01) >> 1;
  initial begin #1 $display("r=%h", r); $finish; end
endmodule
