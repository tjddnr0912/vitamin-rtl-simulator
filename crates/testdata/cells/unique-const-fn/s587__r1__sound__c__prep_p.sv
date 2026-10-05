module top;
  function automatic int f(input int a);
    if (a == 1) return 1;
    return 7;
  endfunction
  logic [31:0] y;
  initial begin y = {f(2){1'b1}}; $display("y=%h", y); #1 $finish; end
endmodule
