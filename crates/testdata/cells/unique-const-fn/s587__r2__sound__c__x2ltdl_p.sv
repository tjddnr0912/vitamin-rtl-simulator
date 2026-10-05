module top;
  typedef logic [3:0] T;
  function automatic logic [31:0] f(input int a);
    T t;
    if (a == 1) t = 1;
    return {28'd0, t};
  endfunction
  localparam logic [31:0] P = f(2);
  initial begin $display("P=%h", P); #1 $finish; end
endmodule
