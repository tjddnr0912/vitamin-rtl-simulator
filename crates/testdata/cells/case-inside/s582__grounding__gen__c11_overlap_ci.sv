module top;
  logic [3:0] v; int m;
  initial begin
    v = 4'd5; m = 9;
    case (v) inside
      [4'd0:4'd7]: m = 1;
      4'd5: m = 2;
      4'b01??: m = 3;
      4'b1???: m = 4;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'd6; m = 9;
    case (v) inside
      [4'd0:4'd7]: m = 1;
      4'd5: m = 2;
      4'b01??: m = 3;
      4'b1???: m = 4;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'd8; m = 9;
    case (v) inside
      [4'd0:4'd7]: m = 1;
      4'd5: m = 2;
      4'b01??: m = 3;
      4'b1???: m = 4;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'd9; m = 9;
    case (v) inside
      [4'd0:4'd7]: m = 1;
      4'd5: m = 2;
      4'b01??: m = 3;
      4'b1???: m = 4;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    #10 $finish;
  end
endmodule
