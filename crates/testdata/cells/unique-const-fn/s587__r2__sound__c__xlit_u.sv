module top;
  function automatic logic [3:0] f(input int a);
    unique if (a == 1) return 4'd1;
    return 4'bx;
  endfunction
  localparam logic [3:0] P = f(2);
  initial begin $display("P=%b", P); #1 $finish; end
endmodule
