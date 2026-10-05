module top;
  function automatic logic [31:0] f(input int a);
    logic [3:0] t;
    unique if (a == 1) t = 4'd1;
    t[0] = 1'b1;
    return {28'd0, 4'(t[2:1])};
  endfunction
  localparam logic [31:0] P = f(2);
  initial begin $display("P=%h", P); #1 $finish; end
endmodule
