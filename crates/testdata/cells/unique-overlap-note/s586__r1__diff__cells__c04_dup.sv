module top;
  int x = 2; int y1, y2, y3, y4, y5; logic [3:0] w = 4'b1010;
  initial #100 $finish;
  function automatic int f(int v);
    unique case (v)
      2: return 10;
      1, 2: return 20;
      default: return 30;
    endcase
  endfunction
  always_comb begin
    unique case (x)
      2: y3 = 1;
      2: y3 = 2;
      default: y3 = 3;
    endcase
  end
  initial begin
    #1;
    unique case (x)
      1, 2, 3: y1 = 1;
      2: y1 = 2;
      default: y1 = 3;
    endcase
    unique0 casex (w)
      4'b1x1x: y2 = 1;
      4'b1010: y2 = 2;
      4'bxxxx: y2 = 3;
    endcase
    unique case (1'b1)
      w[1]: y4 = 1;
      w[3]: y4 = 2;
    endcase
    unique case (x)
      0: y5 = 10; 1: y5 = 11; 2: y5 = 1; 3: y5 = 13; 4: y5 = 14; 5: y5 = 15;
      6: y5 = 16; 7: y5 = 17; 8: y5 = 18; 9: y5 = 19; 10: y5 = 20; 2: y5 = 2;
    endcase
    $display("y1=%0d y2=%0d y3=%0d y4=%0d y5=%0d f=%0d", y1, y2, y3, y4, y5, f(2));
  end
endmodule
