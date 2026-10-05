module top;
  function int f(input int x); return x; endfunction
  function logic signed [3:0] g(input int x); return x[3:0]; endfunction
  int r;
  initial begin
    #10 $finish;
  end
  initial begin
    case (f(-1)) 4'sb1111: r = 1; default: r = 0; endcase
    $display("a r=%0d", r);
    case (g(-1)) 8'sb1111_1111: r = 1; default: r = 0; endcase
    $display("b r=%0d", r);
    case (f(-1)) -1: r = 1; default: r = 0; endcase
    $display("c r=%0d", r);
    r = (f(-1) == 4'sb1111);
    $display("d r=%0d", r);
    r = (g(-1) == 8'sb1111_1111);
    $display("e r=%0d", r);
    $finish;
  end
endmodule
