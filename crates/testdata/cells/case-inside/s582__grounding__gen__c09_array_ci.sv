module top;
  logic [3:0] v; int m; logic [3:0] arr [0:1] = '{4'd5, 4'd7};
  initial begin
    v = 4'd5; m = 9;
    case (v) inside
      arr: m = 1;
      4'd2: m = 2;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'd7; m = 9;
    case (v) inside
      arr: m = 1;
      4'd2: m = 2;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'd2; m = 9;
    case (v) inside
      arr: m = 1;
      4'd2: m = 2;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'd6; m = 9;
    case (v) inside
      arr: m = 1;
      4'd2: m = 2;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    #10 $finish;
  end
endmodule
