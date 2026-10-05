module top;
  function automatic logic [3:0] f(input int a);
    logic [3:0] t = 'x;
    if (a == 1) t = 4'd1;
    return t;
  endfunction
  localparam logic [3:0] P = f(2);
  initial begin $display("P=%b", P); #1 $finish; end
endmodule
