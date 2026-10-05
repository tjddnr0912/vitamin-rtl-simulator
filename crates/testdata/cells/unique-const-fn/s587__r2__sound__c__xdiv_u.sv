module top;
  function automatic logic [3:0] f(input int a);
    logic [3:0] d;
    d = 4'd0;
    unique if (a == 1) return 4'd1;
    return 4'd8 / d;
  endfunction
  localparam logic [3:0] P = f(2);
  initial begin $display("P=%b", P); #1 $finish; end
endmodule
