module top;
  typedef union tagged { void Invalid; int Valid; } VInt;
  VInt x; int m;
  initial begin
    x = tagged Valid 5;
    case (x) matches
      tagged Invalid: m = 0;
      tagged Valid .n: m = n;
    endcase
    $display("m=%0d", m);
    #10 $finish;
  end
endmodule
