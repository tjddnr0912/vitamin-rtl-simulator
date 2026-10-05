module top;
  function automatic logic [31:0] f(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd1;
    t += 4'd2;
    return {28'd0, 4'(t)};
  endfunction
  localparam logic [31:0] P = f(2);
  initial begin $display("P=%h", P); #1 $finish; end
endmodule
