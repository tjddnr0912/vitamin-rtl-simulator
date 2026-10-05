module top;
  function automatic logic [3:0] f(input int a);
    logic [3:0] t;
    t = 4'b1010;
    if (a == 1) return 4'd1;
    return {3'b000, t[a + 7]};
  endfunction
  localparam logic [3:0] P = f(2);
  initial begin $display("P=%b", P); #1 $finish; end
endmodule
