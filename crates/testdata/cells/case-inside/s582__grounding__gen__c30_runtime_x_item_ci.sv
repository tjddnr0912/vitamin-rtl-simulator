module top;
  logic [3:0] v; int m; logic [3:0] w = 4'b1x00;
  initial begin
    v = 4'b1000; m = 9;
    case (v) inside
      w: m = 1;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'b1100; m = 9;
    case (v) inside
      w: m = 1;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'b0100; m = 9;
    case (v) inside
      w: m = 1;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    #10 $finish;
  end
endmodule
